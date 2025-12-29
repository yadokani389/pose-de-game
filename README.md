# pose-de-game

体の動きで操作するゲームのプロジェクトです。`detect/` がカメラ入力から姿勢推定を行い、`game/` が受信した姿勢情報でゲームを動かします。送受信はローカルの TCP または Unix domain socket を使用します。

## 構成
- `detect/`: Python で姿勢推定（YOLO）を行い、姿勢情報を CBOR で送信
- `game/`: Rust + Bevy のゲーム本体。受信した姿勢情報で操作

## データフロー概要
1. `detect` がカメラからフレーム取得
2. 姿勢推定結果を正規化して CBOR でシリアライズ
3. ソケット経由で `game` に送信
4. `game` が受信し、入力や描画に反映

## 実行方法
### 1) ゲーム側（受信）
```bash
cd game
cargo run --release
```

デバッグ用に人物表示を行う場合:
```bash
cd game
cargo run --release -- --show-person
```

### 2) 検出側（送信）
```bash
cd detect
python main.py
```

uv を使う場合:
```bash
cd detect
uv run python main.py
```

デバッグ用に人物 PNG を送信する場合:
```bash
cd detect
python main.py --send-person-png
```

## 送受信の設定
両方とも `--transport` で `tcp` / `unix` を指定できます。未指定時は OS に応じて自動選択されます。

例（TCPで接続する場合）:
```bash
cd game
cargo run --release -- --transport tcp --tcp-addr 127.0.0.1:45233
```

```bash
cd detect
python main.py --transport tcp --tcp-addr 127.0.0.1:45233
```

## ライセンス
MIT / Apache-2.0
