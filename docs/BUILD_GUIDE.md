# TUFF-BRG-ECO Build & Setup Guide

## 1. Rust ビルド
```bash
cd /mnt/thpdoc/Develop/TUFF-BRG-ECO
cargo build -p tuff_brg
cargo build -p transformer_neo
```

## 2. WAL -> History 生成
```bash
# WAL の既定位置を使用
TUFF_WAL_PATH=_tuffdb/tuff.wal TUFF_HISTORY_OUT=history_out \
  cargo run -p transformer_neo --bin history_compile
```

## 3. MID 起動
```bash
TUFF_STOP_CONFIDENCE=0.35 \
  cargo run -p tuff_brg
```

## 4. ブラウザ拡張の読み込み (Chrome)
1. `chrome://extensions/` を開く
2. 右上の「デベロッパーモード」をON
3. 「パッケージ化されていない拡張機能を読み込む」
4. `tuff-brg/addon/` を選択

## 5. Options Page 初期設定
1. 拡張機能の「詳細」→「拡張機能のオプション」を開く
2. `TUFF_WEB_BASE` に `http://127.0.0.1:8787` を入力して保存
3. （任意）AI_ORIGIN を設定する場合は拡張のストレージに保存

## 6. Smoke Test Checklist
```bash
# 1) ポートが開いているか
ss -ltnp | grep 8787

# 2) History Viewer が返るか
curl -i http://127.0.0.1:8787/history

# 3) JSON API が返るか
curl -i http://127.0.0.1:8787/history/api/latest
curl -i http://127.0.0.1:8787/history/api/timeline
```

## 7. 既知の注意点
- `history_out/` が存在しない場合、/history/api/* は 404 を返します。
- 拡張機能は `localhost` 前提のため、同一マシンでの起動が前提です。
