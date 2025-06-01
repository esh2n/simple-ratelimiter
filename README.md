# Simple Rate Limiter

## 実装アルゴリズム

### 1. トークンバケット (Token Bucket)
- バケツに一定レートでトークンを追加
- リクエスト時にトークンを消費
- バースト対応可能、平滑化なし

### 2. リーキーバケット (Leaky Bucket)
- バケツから一定レートで水が漏れる
- リクエストは水として追加
- トラフィックの平滑化

### 3. 固定ウィンドウカウンタ (Fixed Window Counter)
- 固定時間枠でカウント
- シンプル・高速
- 境界問題あり

### 4. スライディングウィンドウログ (Sliding Window Log)
- 全リクエストのタイムスタンプ記録
- 最も正確
- メモリ使用量が多い

### 5. スライディングウィンドウカウンタ (Sliding Window Counter)
- 固定ウィンドウとスライディングの組み合わせ
- メモリ効率と精度のバランス

## TypeScript実装

```bash
cd typescript
npm install
npm run dev
```

## Go実装

```bash
cd go
go mod download
go run main.go
```

## Rust実装

```bash
cd rust
cargo build --release
cargo run
```

## API エンドポイント

### データ取得（レート制限対象）
```
GET /api/data/{algorithm}
```

利用可能なアルゴリズム:
- `token-bucket`
- `leaky-bucket`
- `fixed-window`
- `sliding-window-log`
- `sliding-window-counter`

### 統計情報
```
GET /api/stats/{algorithm}
```

### アルゴリズム一覧
```
GET /api/algorithms
```

## レスポンスヘッダー

レート制限情報は以下のヘッダーで返されます：
- `X-RateLimit-Limit`: 制限数
- `X-RateLimit-Remaining`: 残りリクエスト数
- `X-RateLimit-Reset`: リセット時刻（ISO 8601形式）

## 使用例

```bash
# トークンバケットアルゴリズムでリクエスト
curl http://localhost:3000/api/data/token-bucket

# 統計情報を確認
curl http://localhost:3000/api/stats/token-bucket

# 利用可能なアルゴリズム一覧
curl http://localhost:3000/api/algorithms
```

## レート制限設定

各実装で以下の設定を使用：
- 制限数: 10リクエスト
- ウィンドウサイズ: 60秒（固定ウィンドウ、スライディングウィンドウ）
- トークン補充: 1秒に1トークン（トークンバケット）
- リーク率: 1秒に10リクエスト（リーキーバケット）