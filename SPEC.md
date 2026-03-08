# RAWdevRust — 技術仕様書

## 概要

RustでRAW現像を行うCLIツール。  
CanonのCR2をはじめとする主要RAWフォーマットに対応し、RCDデモザイクを中心とした独自の現像パイプラインを実装する。

**リリース目標**: まず v1.0 を CLIツール として完成させ、その後 WebUI 対応を行う。

---

## アーキテクチャ

```
RAW File
  └─ [rawler] CR2/NEF/ARW/ORF etc. デコード
       ↓
  ベイヤー配列 + メタデータ（ブラックレベル・WB係数・カメラ行列）
       ↓
  ブラックレベル減算 & スケーリング
       ↓
  RCDデモザイク（自前実装）
       ↓
  ホワイトバランス適用
       ↓
  カメラRGB → XYZ → sRGB 変換（DCPプロファイル対応）
       ↓
  現像調整（露出・コントラスト・トーンカーブ etc.）
       ↓
  出力（PPM / TIFF / JPEG）
```

---

## コンポーネント構成

### `raw_decode` — RAWデコード

- クレート: [`rawler`](https://github.com/dnglab/dnglab)（純Rust実装）
- 担当: Lossless JPEG 展開、ベイヤー配列の取り出し、メタデータ取得
- 対応フォーマット: CR2, NEF, ARW, ORF（rawler対応フォーマット全般）

### `demosaic` — デモザイク

メインアルゴリズム: **RCD (Ratio Corrected Demosaicing)**

RCDの手順:
1. Hamilton-Adams EdgeDirected 補間で緑チャネルを補完
2. `R/G`・`B/G` の比率マップを生成
3. 比率マップを平滑化（アーティファクト抑制）
4. 比率 × 補間済み緑チャネルで赤・青を復元

将来的な拡張:
- **RCD + VNG4 ブレンド**: RCDの解像感とVNG4の滑らかさを合成（RawTherapee方式）
- **AMaZE**: オプションとして追加（高品質・低速）
- **Bilinear**: デバッグ・比較用フォールバック

参考: W. Lim, J. Ding (2018) *"Ratio Corrected Demosaicing"*

### `color` — カラー変換

- ホワイトバランス係数の適用
- カメラRGB → XYZ → sRGB 3×3行列変換
- DCPプロファイル読み込み・適用（カラーマトリクス / HSLマップ / トーンカーブ）

### `develop` — 現像パラメータ

- 露出補正
- コントラスト・明るさ
- 彩度・色相
- トーンカーブ（将来対応）

### `output` — 出力

- PPM（デバッグ用）
- TIFF（16bit対応予定）
- JPEG

---

## 技術スタック

| 役割 | 技術 |
|---|---|
| 言語 | Rust |
| RAWデコード | `rawler` |
| CLIインターフェース | `clap` |
| 行列演算 | `nalgebra`（予定） |
| JPEG出力 | `image` クレート |
| WebUI（将来） | WASM + WebGPU |

---

## 開発フェーズ

### v1.0 — CLIツール

| Phase | 内容 |
|---|---|
| 1 | プロジェクトセットアップ・RAWデコード + Bilinear デモザイク（動作確認） |
| 2 | **RCDデモザイク実装** |
| 3 | ホワイトバランス・カラーマトリクス適用 |
| 4 | DCPプロファイル対応 |
| 5 | CLIインターフェース整備・出力フォーマット対応（TIFF/JPEG） |

### v2.0 以降 — WebUI

| Phase | 内容 |
|---|---|
| 6 | WASM対応（Rustコアをブラウザで動作させる） |
| 7 | WebGPU UIの実装 |

---

## 設計方針

- **デコードと現像の分離**: RAWデコードはrawlerに任せ、デモザイク以降は完全に自前実装
- **フォーマット非依存**: rawlerの抽象化レイヤーを使い、CR2以外も同一パイプラインで処理
- **段階的高品質化**: まずRCD単体、次にRCD+VNG4ブレンド、最終的にAMaZEオプション
