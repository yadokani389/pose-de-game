import io
import socket
from typing import Dict

import cbor2
import cv2
import numpy as np
from PIL import Image
from ultralytics import YOLO


def compute_iou(box_a: np.ndarray, box_b: np.ndarray) -> float:
    """Compute IoU between two boxes defined as [x1, y1, x2, y2]."""
    xa1, ya1, xa2, ya2 = box_a
    xb1, yb1, xb2, yb2 = box_b

    inter_x1 = max(xa1, xb1)
    inter_y1 = max(ya1, yb1)
    inter_x2 = min(xa2, xb2)
    inter_y2 = min(ya2, yb2)

    inter_w = max(0.0, inter_x2 - inter_x1)
    inter_h = max(0.0, inter_y2 - inter_y1)
    inter_area = inter_w * inter_h

    area_a = max(0.0, xa2 - xa1) * max(0.0, ya2 - ya1)
    area_b = max(0.0, xb2 - xb1) * max(0.0, yb2 - yb1)

    denom = area_a + area_b - inter_area
    if denom <= 0.0:
        return 0.0
    return inter_area / denom


def match_boxes(pose_boxes: np.ndarray, seg_boxes: np.ndarray) -> Dict[int, int]:
    """Match pose detections with segmentation masks using IoU."""
    assignments: Dict[int, int] = {}
    used = set()

    for pose_idx, pose_box in enumerate(pose_boxes):
        best_iou = 0.0
        best_idx = None
        for seg_idx, seg_box in enumerate(seg_boxes):
            if seg_idx in used:
                continue
            iou = compute_iou(pose_box, seg_box)
            if iou > best_iou:
                best_iou = iou
                best_idx = seg_idx

        if best_idx is not None and best_iou > 0.1:
            assignments[pose_idx] = best_idx
            used.add(best_idx)

    return assignments


def mask_to_png(image_bgr: np.ndarray, mask: np.ndarray) -> bytes:
    """Apply the given mask to the image and return the transparent PNG bytes."""
    binary_mask = mask.astype(bool)
    rgba = cv2.cvtColor(image_bgr, cv2.COLOR_BGR2RGBA)
    alpha = (binary_mask.astype(np.uint8)) * 255
    rgba[..., 3] = alpha
    rgba[alpha == 0, :3] = 0

    buf = io.BytesIO()
    Image.fromarray(rgba).save(buf, format="PNG")
    return buf.getvalue()

pose = YOLO("./yolo11n-pose.pt")
seg = YOLO("./yolo11n-seg.pt")
cap = cv2.VideoCapture(0)

while cap.isOpened():
    success, image = cap.read()
    if not success:
        continue

    results = pose(image)[0]
    seg_results = seg(image)[0]

    if results.keypoints is None:
        continue

    # Extract keypoints and confidence for all people at once
    xy = results.keypoints.xy.cpu().numpy()  # Shape: (n_people, 17, 2)
    conf = (
        results.keypoints.conf.cpu().numpy()
        if results.keypoints.conf is not None
        else np.ones((len(xy), 17))
    )

    # Check if keypoints have the expected shape (17 joints)
    if xy.shape[-2] == 0:
        continue

    # Vectorized processing
    valid_mask = (conf > 0.8) & (xy[:, :, 0] > 1.0) & (xy[:, :, 1] > 1.0)

    # Get image dimensions for normalization
    h, w = image.shape[:2]

    pose_boxes = (
        results.boxes.xyxy.cpu().numpy()
        if results.boxes is not None
        else np.zeros((len(xy), 4))
    )

    seg_boxes = (
        seg_results.boxes.xyxy.cpu().numpy()
        if seg_results.boxes is not None
        else np.zeros((0, 4))
    )

    segmentation_masks = []
    if seg_results.masks is not None:
        for mask in seg_results.masks.data.cpu().numpy():
            resized = cv2.resize(
                mask,
                (w, h),
                interpolation=cv2.INTER_NEAREST,
            )
            segmentation_masks.append(resized > 0.5)

    mask_assignments = (
        match_boxes(pose_boxes, seg_boxes)
        if len(segmentation_masks) == len(seg_boxes)
        else {}
    )

    people_data = []
    for i in range(len(xy)):
        body_keypoints = [
            [float(xy[i, j, 0] / w), float(xy[i, j, 1] / h)]
            if valid_mask[i, j]
            else None
            for j in range(17)
        ]

        png_bytes = None
        seg_idx = mask_assignments.get(i)
        if seg_idx is not None and seg_idx < len(segmentation_masks):
            png_bytes = mask_to_png(image, segmentation_masks[seg_idx])

        people_data.append(
            {
                "keypoints": body_keypoints,
                "right_hand_closed": None,
                "left_hand_closed": None,
                "person_png": png_bytes,
            }
        )

    if people_data:
        try:
            cbor_data = cbor2.dumps(people_data)
            with socket.socket(socket.AF_INET, socket.SOCK_DGRAM) as sock:
                sock.sendto(cbor_data, ("127.0.0.1", 45233))
        except Exception as e:
            print(f"Error: {e}")
