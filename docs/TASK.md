# RAWdevRust — タスク

## Phase 1: プロジェクトセットアップ・RAWデコード + Bilinear デモザイク
- [x] プロジェクト構成・依存クレート追加 (rawler, image, clap, anyhow)
- [x] CR2デコード → ベイヤー配列・メタデータ取り出し (`decode.rs`)
- [x] ブラックレベル減算・スケーリング
- [x] Bilinear デモザイク実装 (`demosaic/bilinear.rs`)
- [x] PPM出力 (`output.rs`)
- [x] **Fix: Optical Black 除去**（`active_area` クロップ実装）

## Phase 2: RCDデモザイク実装
- [x] Hamilton-Adams EdgeDirected 補間（緑チャネル）
- [x] R/G・B/G 比率マップ生成
- [x] 比率マップの平滑化
- [x] 赤・青チャネル復元
- [x] リファレンスPNGと比較・検証（Bilinear比でエッジ偽色の低減を確認）

## Phase 3: ホワイトバランス・カラーマトリクス適用
- [x] カメラホワイトバランス係数の適用
- [x] カメラRGB → XYZ → sRGB 3×3行列変換
  - Fix: cam_to_xyz_normalized 相当の行正規化を実装（白点ずれ解消）

## Phase 4: DCPプロファイル対応
- [x] DCPファイルパース
- [x] カラーマトリクス適用
- [x] HSLマップ（3D LUT）適用
- [x] トーンカーブ適用

## Phase 5: CLIインターフェース整備・出力フォーマット対応
- [ ] clap による引数設計の整備
- [ ] TIFF出力（16bit）
- [ ] JPEG出力
