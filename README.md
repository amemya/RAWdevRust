# RAWdevRust

RustでRAW現像を行うCLIツール。  
RCDデモザイクを中心とした独自の現像パイプラインを実装し、Canon CR2をはじめとする主要RAWフォーマットに対応する。

## 特徴

- **RCDデモザイク** — AMaZEと同等の解像感を高速に実現
- **DCPプロファイル対応** — カメラ固有の色再現を高精度に適用
- **DCP自動探索** — macOS/Windows上のAdobe Camera Rawフォルダからカメラ固有のDCPを自動検出して適用
- **マルチフォーマット対応** — CR2, NEF, ARW, ORF など

## 使い方

`--dcp` オプションを省略した場合は、自動的に対象カメラのプロファイル（Adobe Standard / Camera Standard）を探して適用します。
```bash
rawdev assets/reference/input.cr2 -o assets/output/output.ppm
```

手動で任意のDCPプロファイルを適用する場合：
```bash
rawdev assets/reference/input.cr2 -o assets/output/output.ppm --dcp assets/profiles/camera.dcp
```

## 開発状況

Phase 1〜4 [完了]  
Phase 5 [進行中]  
詳細は [docs/SPEC.md](./docs/SPEC.md) を参照。

## ライセンス

MIT
