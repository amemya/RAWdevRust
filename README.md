# RAWdevRust

RustでRAW現像を行うCLIツール。  
RCDデモザイクを中心とした独自の現像パイプラインを実装し、Canon CR2をはじめとする主要RAWフォーマットに対応する。

## 特徴

- **RCDデモザイク** — AMaZEと同等の解像感を高速に実現
- **DCPプロファイル対応** — カメラ固有の色再現を高精度に適用
- **マルチフォーマット対応** — CR2, NEF, ARW, ORF など

## 使い方（予定）

```bash
rawdev input.cr2 -o output.jpg
rawdev input.cr2 -o output.tiff --profile camera.dcp
```

## 開発状況

現在Phase 1
詳細は [SPEC.md](./SPEC.md) を参照。

## ライセンス

MIT
