# RAWdevRust — タスク

## Phase 1: プロジェクトセットアップ・RAWデコード + Bilinear デモザイク
- [x] プロジェクト構成・依存クレート追加 (rawler, image, clap, anyhow)
- [x] CR2デコード → ベイヤー配列・メタデータ取り出し (`decode.rs`)
- [x] ブラックレベル減算・スケーリング
- [x] Bilinear デモザイク実装 (`demosaic/bilinear.rs`)
- [x] PPM出力 (`output.rs`)
- [x] **Fix: Optical Black 除去**（`active_area` クロップ実装）

## Phase 2: RCDデモザイク実装
- [ ] Hamilton-Adams EdgeDirected 補間（緑チャネル）
- [ ] R/G・B/G 比率マップ生成
- [ ] 比率マップの平滑化
- [ ] 赤・青チャネル復元
- [ ] リファレンスPNGと比較・検証

## Phase 3: ホワイトバランス・カラーマトリクス適用
- [ ] カメラホワイトバランス係数の適用
- [ ] カメラRGB → XYZ → sRGB 3×3行列変換

## Phase 4: DCPプロファイル対応
- [ ] DCPファイルパース
- [ ] カラーマトリクス適用
- [ ] HSLマップ（3D LUT）適用
- [ ] トーンカーブ適用

## Phase 5: CLIインターフェース整備・出力フォーマット対応
- [ ] clap による引数設計の整備
- [ ] TIFF出力（16bit）
- [ ] JPEG出力
