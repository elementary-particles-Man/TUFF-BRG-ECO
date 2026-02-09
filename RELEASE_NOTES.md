# TUFF-BRG-ECO v1.0 (Prototype)

## Summary
TUFF-BRG-ECO v1.0 は、リアルタイムなハルシネーション遮断を目的としたプロトタイプリリースです。Addon -> Bridge -> Lightweight DB の3層構成で、Fast Path と LLM Path のハイブリッド検証を提供します。

## Key Features
- Real-time Hallucination Blocking
  - 受信ストリームを判定し、危険出力には STOP 制御を返却
- Physical Identity Protocol
  - Origin 固定で責任帰属を明確化
- Hybrid Verification (Lightweight + LLM)
  - Lightweight DB の即時照合（Fast Path）
  - 必要時のみ LLM へフォールスルー（LLM Path）

## Highlight: Live Fire Demo
- "Ishiba / Takaichi" シナリオで Fast Path による実時間遮断を検証可能
- `scripts/live_fire_test.sh` で再現

## Requirements
- Browser: Chrome / Brave
- Runtime:
  - Rust toolchain（ローカル実行時）
  - または Docker（配布時）

## Packaging
- Server: `Dockerfile` を利用して `tuff_brg` をコンテナ化
- Addon: `scripts/pack_addon.sh` で `tuff-brg-addon-v{version}.zip` を生成
