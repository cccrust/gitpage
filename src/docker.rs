use bollard::container::LogOutput;
use bollard::exec::{CreateExecOptions, StartExecResults};
use bollard::models::{ContainerCreateBody, HostConfig, PortBinding};
use bollard::query_parameters::{
    CreateContainerOptions, CreateImageOptions, ListContainersOptions,
    RemoveContainerOptions, StopContainerOptions,
};
use bollard::Docker;
use futures_util::StreamExt;
use std::collections::HashMap;

type PortMap = HashMap<String, Option<Vec<PortBinding>>>;

#[derive(Clone)]
pub struct DockerManager {
    pub docker: Docker,
    pub base_image: String,
    pub network: String,
    pub staging_root: String,
    pub apps_root: String,
}

fn name_filter(name: &str) -> HashMap<String, Vec<String>> {
    HashMap::from([("name".to_string(), vec![name.to_string()])])
}

impl DockerManager {
    pub async fn connect(cfg: &crate::config::Config) -> Result<Self, String> {
        let docker = Docker::connect_with_local_defaults()
            .map_err(|e| format!("Failed to connect to Docker: {}", e))?;

        let base_image = cfg.docker.base_image.clone();
        let network = cfg.docker.network.clone();
        let base = std::path::Path::new(&cfg.storage.base_path);
        let staging_root = base.join("staging").to_string_lossy().to_string();
        let apps_root = base.join("apps").to_string_lossy().to_string();

        let _: Vec<_> = docker
            .create_image(
                Some(CreateImageOptions {
                    from_image: Some(base_image.clone()),
                    tag: Some("latest".to_string()),
                    ..Default::default()
                }),
                None,
                None,
            )
            .collect::<Vec<_>>()
            .await;

        Ok(Self {
            docker,
            base_image,
            network,
            staging_root,
            apps_root,
        })
    }

    pub async fn ensure_user_container(&self, username: &str) -> Result<(), String> {
        let name = format!("gitpage-{}", username);
        let apps_host = std::path::Path::new(&self.apps_root).join(username);
        let apps_host_str = apps_host.to_string_lossy().to_string();

        let containers = self
            .docker
            .list_containers(Some(ListContainersOptions {
                all: true,
                filters: Some(name_filter(&name)),
                ..Default::default()
            }))
            .await
            .map_err(|e| format!("list containers: {}", e))?;

        if let Some(existing) = containers.first() {
            let state = existing.state.map(|s| format!("{:?}", s)).unwrap_or_default();
            let status = existing.status.as_deref().unwrap_or("");
            if state == "RUNNING" || status.contains("Up") {
                return Ok(());
            }
            self.docker
                .start_container(&name, None)
                .await
                .map_err(|e| format!("start {}: {}", name, e))?;
            tracing::info!("Started existing container {}", name);
            return Ok(());
        }

        let cfg = ContainerCreateBody {
            image: Some(self.base_image.clone()),
            hostname: Some(username.to_string()),
            env: Some(vec![format!("USERNAME={}", username)]),
            cmd: Some(vec!["sleep".to_string(), "infinity".to_string()]),
            exposed_ports: Some(vec!["22/tcp".to_string()]),
            host_config: Some(HostConfig {
                network_mode: Some(self.network.clone()),
                binds: Some(vec![
                    format!("gitpage-home-{0}:/home/{0}", username),
                    format!("{}:/workspace", apps_host_str),
                ]),
                port_bindings: Some({
                    let mut m: PortMap = HashMap::new();
                    m.insert("22/tcp".into(), None);
                    m
                }),
                ..Default::default()
            }),
            ..Default::default()
        };

        self.docker
            .create_container(
                Some(CreateContainerOptions {
                    name: Some(name.clone()),
                    platform: String::new(),
                }),
                cfg,
            )
            .await
            .map_err(|e| format!("create {}: {}", name, e))?;

        self.docker
            .start_container(&name, None)
            .await
            .map_err(|e| format!("start {}: {}", name, e))?;

        tracing::info!("Created container {}", name);
        Ok(())
    }

    pub async fn get_container_ip(&self, username: &str) -> Result<String, String> {
        let name = format!("gitpage-{}", username);
        let containers = self
            .docker
            .list_containers(Some(ListContainersOptions {
                all: false,
                filters: Some(name_filter(&name)),
                ..Default::default()
            }))
            .await
            .map_err(|e| format!("list containers: {}", e))?;

        let container = containers
            .first()
            .ok_or_else(|| format!("container {} not found", name))?;

        let nets = container
            .network_settings
            .as_ref()
            .and_then(|ns| ns.networks.as_ref())
            .ok_or("no networks")?;

        for net in nets.values() {
            if let Some(ip) = &net.ip_address {
                if !ip.is_empty() && ip != "0.0.0.0" {
                    return Ok(ip.clone());
                }
            }
        }
        Err(format!("no IP for {}", name))
    }

    pub async fn exec_command(
        &self,
        username: &str,
        cmd: &[&str],
        workdir: Option<&str>,
    ) -> Result<String, String> {
        let name = format!("gitpage-{}", username);

        let exec_opts = CreateExecOptions {
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            cmd: Some(cmd.iter().map(|s| s.to_string()).collect()),
            working_dir: workdir.map(|s| s.to_string()),
            ..Default::default()
        };

        let exec = self
            .docker
            .create_exec(&name, exec_opts)
            .await
            .map_err(|e| format!("create exec: {}", e))?;

        let output = self
            .docker
            .start_exec(&exec.id, None)
            .await
            .map_err(|e| format!("start exec: {}", e))?;

        let mut result = String::new();
        if let StartExecResults::Attached { mut output, .. } = output {
            while let Some(chunk) = output.next().await {
                match chunk {
                    Ok(LogOutput::StdOut { message })
                    | Ok(LogOutput::StdErr { message }) => {
                        result.push_str(&String::from_utf8_lossy(&message));
                    }
                    _ => {}
                }
            }
        }
        Ok(result)
    }

    pub async fn exec_build(
        &self,
        username: &str,
        repo_name: &str,
        cmd: &str,
    ) -> Result<String, String> {
        let workdir = format!("/workspace/{}/source", repo_name);
        self.exec_command(username, &["sh", "-c", cmd], Some(&workdir)).await
    }

    pub async fn exec_start_detached(
        &self,
        username: &str,
        repo_name: &str,
        cmd: &str,
        port: u16,
        env_vars: Option<Vec<String>>,
    ) -> Result<(), String> {
        let name = format!("gitpage-{}", username);
        let workdir = format!("/workspace/{}/source", repo_name);

        let mut env = vec![
            format!("PORT={}", port),
            "HOST=0.0.0.0".to_string(),
        ];
        if let Some(vars) = env_vars {
            env.extend(vars);
        }

        let exec_opts = CreateExecOptions {
            attach_stdout: Some(false),
            attach_stderr: Some(false),
            attach_stdin: Some(false),
            cmd: Some(vec!["sh".to_string(), "-c".to_string(), cmd.to_string()]),
            working_dir: Some(workdir),
            env: Some(env),
            ..Default::default()
        };

        let exec = self
            .docker
            .create_exec(&name, exec_opts)
            .await
            .map_err(|e| format!("create exec: {}", e))?;

        match self
            .docker
            .start_exec(&exec.id, Some(bollard::exec::StartExecOptions {
                detach: true,
                tty: false,
                output_capacity: None,
            }))
            .await
        {
            Ok(StartExecResults::Detached) => Ok(()),
            Ok(_) => Err("unexpected attached result".into()),
            Err(e) => Err(format!("start exec: {}", e)),
        }
    }

    pub async fn exec_check_status(
        &self,
        username: &str,
        repo_name: &str,
        port: u16,
    ) -> Result<bool, String> {
        let check = self.exec_command(
            username,
            &[
                "sh", "-c",
                &format!("lsof -i :{} -t 2>/dev/null | head -1", port),
            ],
            Some(&format!("/workspace/{}/source", repo_name)),
        ).await?;
        Ok(!check.trim().is_empty())
    }

    pub async fn exec_stop_app(
        &self,
        username: &str,
        port: u16,
    ) -> Result<(), String> {
        self.exec_command(
            username,
            &["sh", "-c", &format!("lsof -ti :{} | xargs -r kill -9", port)],
            None,
        ).await?;
        Ok(())
    }

    #[allow(dead_code)]
    pub async fn remove_container(&self, username: &str) -> Result<(), String> {
        let name = format!("gitpage-{}", username);
        self.docker
            .stop_container(&name, None::<StopContainerOptions>)
            .await
            .ok();
        self.docker
            .remove_container(&name, Some(RemoveContainerOptions {
                v: true,
                force: true,
                link: false,
            }))
            .await
            .map_err(|e| format!("remove container: {}", e))
    }

    #[allow(dead_code)]
    pub async fn list_user_containers(&self) -> Result<Vec<String>, String> {
        let containers = self
            .docker
            .list_containers(Some(ListContainersOptions {
                all: true,
                filters: Some(HashMap::from([(
                    "name".to_string(),
                    vec!["gitpage-".to_string()],
                )])),
                ..Default::default()
            }))
            .await
            .map_err(|e| format!("list containers: {}", e))?;

        Ok(containers
            .iter()
            .filter_map(|c| {
                c.names
                    .as_ref()
                    .and_then(|names| names.first().cloned())
            })
            .collect())
    }
}
