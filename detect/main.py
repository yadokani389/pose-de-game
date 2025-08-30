import socket

import cbor2
import cv2
import numpy as np
from ultralytics import YOLO

pose = YOLO("./yolo11n-pose.pt")
cap = cv2.VideoCapture(0)

while cap.isOpened():
    success, image = cap.read()
    if not success:
        continue

    results = pose(image)[0]

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

    people_data = []
    for i in range(len(xy)):
        body_keypoints = [
            [float(xy[i, j, 0] / w), float(xy[i, j, 1] / h)]
            if valid_mask[i, j]
            else None
            for j in range(17)
        ]
        people_data.append((body_keypoints, None, None))

    if people_data:
        try:
            print(people_data)
            cbor_data = cbor2.dumps(people_data)
            with socket.socket(socket.AF_INET, socket.SOCK_DGRAM) as sock:
                sock.sendto(cbor_data, ("127.0.0.1", 45233))
        except Exception as e:
            print(f"Error: {e}")
