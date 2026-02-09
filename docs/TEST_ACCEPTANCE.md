# TUFF-BRG-ECO 受け入れ基準（v1.0系）

## 目的
配布前に、システムが「誤情報遮断」「フォールバック継続」「復旧可能性」「基本性能」を満たすことを確認する。

## テスト観点
- 正常系
- 幻覚遮断
- フォールバック
- 障害復旧
- 性能

## 受け入れ条件

### 1. 正常系
- 条件:
  - Bridge が `127.0.0.1:8787` で起動する。
  - StreamFragment を受信し、JudgeResult を返す。
- 合格基準:
  - `scripts/regression_check.sh` が `OK: regression check passed` を返す。
  - `_tuffdb/tuff.wal` が作成済みまたは追記される。

### 2. 幻覚遮断（Live Fire）
- 条件:
  - `meaning.db` を使った Fast Path が有効。
  - 幻覚入力: `prime-minister\t...Shigeru Ishiba...`
- 合格基準:
  - `JudgeResult.status` が `SMOKE` または `GRAY_BLACK`。
  - `ControlCommand.command` が `STOP`。
  - `scripts/live_fire_test.sh` が `OK: live fire test passed` を返す。

### 3. フォールバック
- 条件:
  - Lightweight 判定が `Unknown` の入力を送る。
- 合格基準:
  - 処理が失敗せず LLM Path にフォールスルーする。
  - `reason` に `source=LLM` が含まれる JudgeResult が返る。

### 4. 障害復旧（WAL）
- 条件:
  - WAL末尾を意図的に破損させて再起動する。
- 合格基準:
  - `TUFF_WAL_RECOVERY_MODE=truncate` で起動継続する。
  - 破損尾部が切り詰められ、以降の追記が可能。
  - `TUFF_WAL_RECOVERY_MODE=strict` では起動失敗する（期待動作）。

### 5. 性能（簡易）
- 条件:
  - Fast Path Hit の連続入力を実行。
- 合格基準:
  - Bridge プロセスがハングしない。
  - 連続入力中に WS 応答が継続し、強制停止を要しない。

## 実行コマンド
```bash
# 主要スイート（ローカル）
scripts/test_suite.sh

# 個別
scripts/regression_check.sh
scripts/live_fire_test.sh
```

## 非機能上の注意
- `dist/` と `_tuffdb/` は成果物・実行データであり、Git追跡対象外。
- 本基準は v1.0 プロトタイプ向け。正式リリース時に負荷試験を拡張する。
