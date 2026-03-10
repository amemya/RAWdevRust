# RAWdevRust

RustでRAW現像を行うCLIツール。  
RCDデモザイクを中心とした独自の現像パイプラインを実装し、Canon CR2をはじめとする主要RAWフォーマットに対応する。

## 特徴

- **RCDデモザイク** — AMaZEと同等の解像感を高速に実現
- **DCPプロファイル対応** — カメラ固有の色再現を高精度に適用
- **マルチフォーマット対応** — CR2, NEF, ARW, ORF など

## 使い方

```bash
rawdev assets/reference/input.cr2 -o assets/output/output.jpg
rawdev assets/reference/input.cr2 -o assets/output/output.tiff --profile assets/profiles/camera.dcp
```

## 開発状況

Phase 1 [完了]  
Phase 2 [完了]  
Phase 3 [完了]  
現在Phase 4  
詳細は [docs/SPEC.md](./docs/SPEC.md) を参照。

## ライセンス

MIT
