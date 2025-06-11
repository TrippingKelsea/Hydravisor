#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct ProvidersConfig {
    #[serde(default)]
    pub ollama: OllamaConfig,
    #[serde(default)]
    pub bedrock: BedrockConfig,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct SshConfig {
    #[serde(default)]
    pub hosts: HashMap<String, SshHostConfigEntry>, // Key is host alias, e.g., "foo-vm"
    #[serde(default, rename = "vm")]
    pub vm_specific_configs: HashMap<String, VmSshDetails>, // Key is vm-name

    #[serde(skip)]
    pub source_path: Option<String>,
} 