# pose-de-game

体の動きで操作するゲームのプロジェクトです。Rust だけでカメラ入力から姿勢推定まで行い、推論結果でゲームを動かします。

## 構成

- `src/infer/`: 姿勢推論パイプライン（前処理/推論/後処理）
- `src/pose/runtime/`: カメラ取得・推論実行・ゲーム用データ変換
- `src/pose/visualize.rs`: デバッグ表示（人物画像/手の位置）
- `src/games/`: ゲーム本体（Bevy）

## データフロー概要

1. カメラからフレーム取得
2. Rust で姿勢推定（ONNX/ORT/OpenVINO）
3. 推論結果を正規化してゲーム入力・描画に反映

## 実行方法

```bash
cargo run --release
```

デバッグ用に人物表示を行う場合（セグメンテーションも有効化）:

```bash
cargo run --release -- --show-person
```

利用可能なカメラ一覧:

```bash
cargo run --release -- --list-cameras
```

モデルパスを明示する場合（未指定なら `assets/models` の埋め込みモデルを使用）:

```bash
cargo run --release -- --pose-model path/to/yolo11n-pose.onnx --seg-model path/to/yolo11n-seg.onnx
```

バックエンド指定例（`openvino` は feature 有効時のみ）:

```bash
cargo run --release -- --backend ort --require-cuda
```

## 主なオプション

- `--list-cameras`: カメラ一覧を表示
- `--camera <index>`: 使用カメラ指定
- `--pose-model <path>` / `--seg-model <path>`: モデルパス指定（未指定時は埋め込み）
- `--backend <onnx|ort|openvino>`: 推論バックエンド
- `--show-person`: 人物画像のデバッグ表示（セグメンテーション有効化）
- `--profile`: 推論の平均時間をログ出力
- `--require-cuda`: ORT の CUDA 必須指定

## ライセンス

MIT / Apache-2.0
