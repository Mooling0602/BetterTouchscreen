// 配置文件自动创建测试

use bettertouchscreen::config::Config;
use std::fs;
use tempfile::tempdir;

#[test]
fn config_auto_creation_default_path() {
    // 创建临时目录
    let dir = tempdir().unwrap();
    let config_dir = dir.path().join(".config").join("bettertouchscreen");
    let config_path = config_dir.join("config.toml");
    
    // 模拟默认路径
    let path_str = config_path.to_string_lossy().to_string();
    
    // 加载配置（应该自动创建）
    let config = Config::load(Some(&path_str)).unwrap();
    
    // 验证配置文件已创建
    assert!(config_path.exists(), "配置文件应该被自动创建");
    
    // 验证配置文件内容
    let content = fs::read_to_string(&config_path).unwrap();
    assert!(content.contains("scroll_threshold"), "配置文件应包含scroll_threshold");
    assert!(content.contains("swipe_threshold"), "配置文件应包含swipe_threshold");
    assert!(content.contains("swipe_deadzone"), "配置文件应包含swipe_deadzone");
    assert!(content.contains("pointer_sensitivity"), "配置文件应包含pointer_sensitivity");
    assert!(content.contains("scroll_sensitivity"), "配置文件应包含scroll_sensitivity");
    
    // 验证返回的是默认配置
    assert_eq!(config.scroll_threshold, 5.0);
    assert_eq!(config.swipe_threshold, 50.0);
    assert_eq!(config.swipe_deadzone, 30.0);
    assert_eq!(config.pointer_sensitivity, 0.5);
    assert_eq!(config.scroll_sensitivity, 0.05);
}

#[test]
fn config_auto_creation_with_parent_dir() {
    // 测试父目录不存在的情况
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("deep").join("nested").join("config.toml");
    
    let path_str = config_path.to_string_lossy().to_string();
    
    // 加载配置（应该自动创建父目录和配置文件）
    let _config = Config::load(Some(&path_str)).unwrap();
    
    // 验证配置文件已创建
    assert!(config_path.exists(), "配置文件应该被自动创建");
    
    // 验证父目录已创建
    let parent_dir = config_path.parent().unwrap();
    assert!(parent_dir.exists(), "父目录应该被自动创建");
}

#[test]
fn config_load_existing_file() {
    // 测试加载已存在的配置文件
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("config.toml");
    
    // 创建自定义配置文件
    let custom_config = r#"
scroll_threshold = 10.0
swipe_threshold = 100.0
swipe_deadzone = 60.0
pointer_sensitivity = 0.8
scroll_sensitivity = 0.1
"#;
    fs::write(&config_path, custom_config).unwrap();
    
    let path_str = config_path.to_string_lossy().to_string();
    
    // 加载配置
    let config = Config::load(Some(&path_str)).unwrap();
    
    // 验证加载的是自定义配置
    assert_eq!(config.scroll_threshold, 10.0);
    assert_eq!(config.swipe_threshold, 100.0);
    assert_eq!(config.swipe_deadzone, 60.0);
    assert_eq!(config.pointer_sensitivity, 0.8);
    assert_eq!(config.scroll_sensitivity, 0.1);
}

#[test]
fn config_load_partial_config() {
    // 测试加载部分配置（使用默认值填充缺失字段）
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("config.toml");
    
    // 创建部分配置文件
    let partial_config = r#"
scroll_threshold = 15.0
# 其他参数省略，使用默认值
"#;
    fs::write(&config_path, partial_config).unwrap();
    
    let path_str = config_path.to_string_lossy().to_string();
    
    // 加载配置
    let config = Config::load(Some(&path_str)).unwrap();
    
    // 验证自定义值和默认值混合
    assert_eq!(config.scroll_threshold, 15.0); // 自定义值
    assert_eq!(config.swipe_threshold, 50.0);  // 默认值
    assert_eq!(config.swipe_deadzone, 30.0);   // 默认值
    assert_eq!(config.pointer_sensitivity, 0.5); // 默认值
    assert_eq!(config.scroll_sensitivity, 0.05); // 默认值
}