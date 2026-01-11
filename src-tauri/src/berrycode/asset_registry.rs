//! Asset Registry - SVGアイコンとビジュアルアセット管理
//! ワークフローノードやUI要素のアイコンを効率的に管理

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// アセットの種類
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum AssetType {
    Icon,
    Image,
    Font,
    Stylesheet,
    Custom(String),
}

/// アセット情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    pub id: String,
    pub name: String,
    pub asset_type: AssetType,
    pub content: String,
    pub mime_type: String,
    pub tags: Vec<String>,
    pub metadata: HashMap<String, String>,
}

/// アセットレジストリ
pub struct AssetRegistry {
    assets: HashMap<String, Asset>,
    cache_dir: PathBuf,
}

impl AssetRegistry {
    /// 新しいAssetRegistryインスタンスを作成
    pub fn new(cache_dir: PathBuf) -> Result<Self> {
        fs::create_dir_all(&cache_dir)?;
        
        let mut registry = Self {
            assets: HashMap::new(),
            cache_dir,
        };

        // デフォルトアイコンセットを登録
        registry.register_default_icons()?;

        Ok(registry)
    }

    /// プロジェクトのアセットレジストリをロード
    pub fn load_from_project(project_path: &Path) -> Result<Self> {
        let cache_dir = project_path.join(".berrycode").join("assets");
        let registry_file = cache_dir.join("registry.json");

        let mut registry = Self::new(cache_dir)?;

        if registry_file.exists() {
            let content = fs::read_to_string(&registry_file)
                .context("Failed to read asset registry")?;
            
            let assets: HashMap<String, Asset> = serde_json::from_str(&content)
                .context("Failed to parse asset registry")?;
            
            registry.assets.extend(assets);
            tracing::info!("📦 Loaded {} assets from project", registry.assets.len());
        }

        Ok(registry)
    }

    /// アセットレジストリを保存
    pub fn save_to_project(&self, project_path: &Path) -> Result<()> {
        let cache_dir = project_path.join(".berrycode").join("assets");
        fs::create_dir_all(&cache_dir)?;
        
        let registry_file = cache_dir.join("registry.json");
        let content = serde_json::to_string_pretty(&self.assets)?;
        
        fs::write(&registry_file, content)?;
        tracing::info!("💾 Saved {} assets to registry", self.assets.len());
        
        Ok(())
    }

    /// アセットを登録
    pub fn register(&mut self, asset: Asset) -> Result<()> {
        let id = asset.id.clone();
        
        // ファイルシステムにキャッシュ
        let cache_path = self.get_cache_path(&id, &asset.asset_type);
        fs::write(&cache_path, &asset.content)?;
        
        self.assets.insert(id.clone(), asset);
        tracing::debug!("✅ Registered asset: {}", id);
        
        Ok(())
    }

    /// アセットを取得
    pub fn get(&self, id: &str) -> Option<&Asset> {
        self.assets.get(id)
    }

    /// アセットを検索（タグベース）
    pub fn search_by_tags(&self, tags: &[String]) -> Vec<&Asset> {
        self.assets
            .values()
            .filter(|asset| {
                tags.iter().any(|tag| asset.tags.contains(tag))
            })
            .collect()
    }

    /// アセットタイプで検索
    pub fn get_by_type(&self, asset_type: &AssetType) -> Vec<&Asset> {
        self.assets
            .values()
            .filter(|asset| &asset.asset_type == asset_type)
            .collect()
    }

    /// アセットを削除
    pub fn remove(&mut self, id: &str) -> Result<()> {
        if let Some(asset) = self.assets.remove(id) {
            let cache_path = self.get_cache_path(&id, &asset.asset_type);
            if cache_path.exists() {
                fs::remove_file(cache_path)?;
            }
            tracing::debug!("🗑️ Removed asset: {}", id);
        }
        Ok(())
    }

    /// キャッシュパスを取得
    fn get_cache_path(&self, id: &str, asset_type: &AssetType) -> PathBuf {
        let extension = match asset_type {
            AssetType::Icon => "svg",
            AssetType::Image => "png",
            AssetType::Font => "ttf",
            AssetType::Stylesheet => "css",
            AssetType::Custom(ext) => ext,
        };
        
        self.cache_dir.join(format!("{}.{}", id, extension))
    }

    /// デフォルトアイコンを登録
    fn register_default_icons(&mut self) -> Result<()> {
        // BerryFlow用のデフォルトアイコンセット
        let icons = vec![
            ("architect", "🏛️", vec!["design", "architecture"]),
            ("ux_designer", "🎨", vec!["design", "ux"]),
            ("ui_designer", "🖼️", vec!["design", "ui"]),
            ("programmer", "💻", vec!["code", "development"]),
            ("test_generator", "🧪", vec!["test", "qa"]),
            ("test_runner", "▶️", vec!["test", "qa"]),
            ("bug_fixer", "🐛", vec!["debug", "fix"]),
            ("refactorer", "♻️", vec!["refactor", "quality"]),
            ("doc_writer", "📝", vec!["documentation"]),
            ("git_commit", "📦", vec!["git", "version-control"]),
            ("workflow", "🔄", vec!["workflow", "automation"]),
            ("success", "✅", vec!["status", "success"]),
            ("error", "❌", vec!["status", "error"]),
            ("warning", "⚠️", vec!["status", "warning"]),
            ("info", "ℹ️", vec!["status", "info"]),
        ];

        for (id, emoji, tags) in icons {
            // エモジをSVGテキストに変換（簡易実装）
            let svg = self.emoji_to_svg(emoji);
            
            let asset = Asset {
                id: id.to_string(),
                name: id.replace('_', " ").to_uppercase(),
                asset_type: AssetType::Icon,
                content: svg,
                mime_type: "image/svg+xml".to_string(),
                tags: tags.into_iter().map(|s| s.to_string()).collect(),
                metadata: HashMap::from([
                    ("emoji".to_string(), emoji.to_string()),
                ]),
            };

            self.register(asset)?;
        }

        tracing::info!("📦 Registered {} default icons", self.assets.len());
        Ok(())
    }

    /// エモジをSVGに変換（簡易実装）
    fn emoji_to_svg(&self, emoji: &str) -> String {
        format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
  <text x="50" y="50" font-size="60" text-anchor="middle" dominant-baseline="central">
    {}
  </text>
</svg>"#,
            emoji
        )
    }

    /// Devicons風のSVGアイコンを登録
    pub fn register_devicon(&mut self, name: &str, svg_content: String) -> Result<()> {
        let asset = Asset {
            id: format!("devicon_{}", name),
            name: name.to_uppercase(),
            asset_type: AssetType::Icon,
            content: svg_content,
            mime_type: "image/svg+xml".to_string(),
            tags: vec!["devicon".to_string(), name.to_string()],
            metadata: HashMap::from([
                ("source".to_string(), "devicons".to_string()),
            ]),
        };

        self.register(asset)
    }

    /// カスタムSVGアイコンを登録
    pub fn register_custom_icon(
        &mut self,
        id: String,
        name: String,
        svg_content: String,
        tags: Vec<String>,
    ) -> Result<()> {
        let asset = Asset {
            id,
            name,
            asset_type: AssetType::Icon,
            content: svg_content,
            mime_type: "image/svg+xml".to_string(),
            tags,
            metadata: HashMap::new(),
        };

        self.register(asset)
    }

    /// すべてのアセットIDを取得
    pub fn list_all(&self) -> Vec<String> {
        self.assets.keys().cloned().collect()
    }

    /// アセット統計を取得
    pub fn get_stats(&self) -> AssetStats {
        let mut type_counts: HashMap<String, usize> = HashMap::new();
        let mut total_size = 0;

        for asset in self.assets.values() {
            let type_name = format!("{:?}", asset.asset_type);
            *type_counts.entry(type_name).or_insert(0) += 1;
            total_size += asset.content.len();
        }

        AssetStats {
            total_count: self.assets.len(),
            type_counts,
            total_size_bytes: total_size,
        }
    }

    /// 重複アセットを検出（コンテンツハッシュベース）
    pub fn find_duplicates(&self) -> HashMap<String, Vec<String>> {
        let mut content_map: HashMap<String, Vec<String>> = HashMap::new();

        for (id, asset) in &self.assets {
            // コンテンツのハッシュを簡易的に計算（実際はSHA256など）
            let hash = format!("{:x}", md5::compute(&asset.content));
            content_map.entry(hash).or_default().push(id.clone());
        }

        // 重複のみ抽出
        content_map
            .into_iter()
            .filter(|(_, ids)| ids.len() > 1)
            .collect()
    }

    /// 未使用アセットを検出
    pub fn find_unused(&self, used_ids: &[String]) -> Vec<String> {
        self.assets
            .keys()
            .filter(|id| !used_ids.contains(id))
            .cloned()
            .collect()
    }

    /// アセットをエクスポート（data URIとして）
    pub fn export_as_data_uri(&self, id: &str) -> Option<String> {
        use base64::Engine;
        self.get(id).map(|asset| {
            let base64 = base64::engine::general_purpose::STANDARD.encode(&asset.content);
            format!("data:{};base64,{}", asset.mime_type, base64)
        })
    }

    /// すべてのアイコンをHTMLドロップダウン用に出力
    pub fn get_icon_options_html(&self) -> String {
        let mut html = String::new();
        
        let mut icons: Vec<_> = self.get_by_type(&AssetType::Icon);
        icons.sort_by_key(|a| &a.name);

        for icon in icons {
            html.push_str(&format!(
                r#"<option value="{}">{} {}</option>"#,
                icon.id,
                icon.metadata.get("emoji").unwrap_or(&String::new()),
                icon.name
            ));
            html.push('\n');
        }

        html
    }
}

/// アセット統計
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetStats {
    pub total_count: usize,
    pub type_counts: HashMap<String, usize>,
    pub total_size_bytes: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_asset_registry_creation() {
        let temp = tempdir().unwrap();
        let registry = AssetRegistry::new(temp.path().to_path_buf()).unwrap();
        
        // デフォルトアイコンが登録されているか
        assert!(registry.assets.len() > 0);
        assert!(registry.get("architect").is_some());
    }

    #[test]
    fn test_search_by_tags() {
        let temp = tempdir().unwrap();
        let registry = AssetRegistry::new(temp.path().to_path_buf()).unwrap();
        
        let results = registry.search_by_tags(&vec!["test".to_string()]);
        assert!(results.len() > 0);
    }

    #[test]
    fn test_emoji_to_svg() {
        let temp = tempdir().unwrap();
        let registry = AssetRegistry::new(temp.path().to_path_buf()).unwrap();
        
        let svg = registry.emoji_to_svg("🏛️");
        assert!(svg.contains("<svg"));
        assert!(svg.contains("🏛️"));
    }

    #[test]
    fn test_custom_icon_registration() {
        let temp = tempdir().unwrap();
        let mut registry = AssetRegistry::new(temp.path().to_path_buf()).unwrap();
        
        let svg = r#"<svg><circle r="10"/></svg>"#.to_string();
        registry
            .register_custom_icon(
                "custom_test".to_string(),
                "Custom Test".to_string(),
                svg.clone(),
                vec!["custom".to_string()],
            )
            .unwrap();
        
        let asset = registry.get("custom_test").unwrap();
        assert_eq!(asset.content, svg);
    }
}
