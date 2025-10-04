use crate::config::Config;
use anyhow::Result;
use futures::stream::StreamExt;
use signal_hook::consts::SIGHUP;
use signal_hook_tokio::Signals;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info};

#[allow(dead_code)]
pub struct ConfigManager {
    config: Arc<RwLock<Config>>,
    config_path: String,
}

#[allow(dead_code)]
impl ConfigManager {
    pub fn new(config: Config, config_path: String) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            config_path,
        }
    }

    pub fn get_config(&self) -> Arc<RwLock<Config>> {
        self.config.clone()
    }

    pub async fn start_reload_watcher(&self) -> Result<()> {
        let config = self.config.clone();
        let config_path = self.config_path.clone();

        // Start signal handler for SIGHUP (reload config)
        #[cfg(unix)]
        {
            let mut signals =
                Signals::new([SIGHUP]).expect("Failed to register SIGHUP signal handler");

            tokio::spawn(async move {
                while let Some(signal) = signals.next().await {
                    if signal == SIGHUP {
                        info!("Received SIGHUP, reloading configuration...");
                        if let Err(e) = Self::reload_config(&config, &config_path).await {
                            error!("Failed to reload configuration: {}", e);
                        }
                    }
                }
            });
        }

        Ok(())
    }

    async fn reload_config(config: &Arc<RwLock<Config>>, config_path: &str) -> Result<()> {
        // Load new config
        let new_config = Config::load(config_path)?;

        // Validate new config
        new_config.validate()?;

        // Hot-swap the configuration
        {
            let mut config_guard = config.write().await;
            *config_guard = new_config;
        }

        info!("Configuration reloaded successfully from {}", config_path);
        Ok(())
    }

    pub async fn watch_file_changes(&self) -> Result<()> {
        let config = self.config.clone();
        let config_path = self.config_path.clone();

        // Simple file modification time watching
        let mut last_modified = std::fs::metadata(&config_path)?.modified()?;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));

            loop {
                interval.tick().await;

                if let Ok(metadata) = std::fs::metadata(&config_path) {
                    if let Ok(modified) = metadata.modified() {
                        if modified > last_modified {
                            info!("Configuration file changed, reloading...");
                            if let Err(e) = Self::reload_config(&config, &config_path).await {
                                error!("Failed to reload configuration: {}", e);
                            } else {
                                last_modified = modified;
                            }
                        }
                    }
                }
            }
        });

        Ok(())
    }
}

// Hot-reloadable configuration validation
#[allow(dead_code)]
impl Config {
    pub fn can_hot_reload(&self, new_config: &Config) -> Result<Vec<String>> {
        let mut warnings = Vec::new();

        // Check if listen addresses changed (requires restart)
        if self.server.listen != new_config.server.listen {
            return Err(anyhow::anyhow!(
                "Listen addresses cannot be changed during hot reload. Restart required."
            ));
        }

        // Check if SSL certificates changed
        for (name, old_vhost) in &self.virtual_hosts {
            if let Some(new_vhost) = new_config.virtual_hosts.get(name) {
                if old_vhost.ssl != new_vhost.ssl {
                    warnings.push(format!("SSL configuration changed for virtual host '{}'. New connections will use new certificates.", name));
                }
            }
        }

        // Check if worker thread count changed
        if self.server.worker_threads != new_config.server.worker_threads {
            warnings.push(
                "Worker thread count changed. This will only affect new connections.".to_string(),
            );
        }

        Ok(warnings)
    }
}
