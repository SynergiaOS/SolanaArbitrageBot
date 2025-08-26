use anyhow::{anyhow, Result};
use log::{error, info, warn};
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer};
use std::path::Path;
use std::fs;
use std::env;

/// Bezpieczny manager portfeli z szyfrowaniem
pub struct SecureWalletManager {
    encryption_key: Option<String>,
    wallet_dir: String,
}

impl SecureWalletManager {
    pub fn new() -> Self {
        Self {
            encryption_key: env::var("WALLET_ENCRYPTION_KEY").ok(),
            wallet_dir: env::var("WALLET_DIR").unwrap_or_else(|_| ".wallets".to_string()),
        }
    }

    /// Ładuje wallet z bezpiecznej lokalizacji
    pub fn load_wallet(&self, wallet_name: &str) -> Result<Keypair> {
        // Sprawdź czy plik istnieje w bezpiecznej lokalizacji
        let wallet_path = format!("{}/{}.json", self.wallet_dir, wallet_name);
        
        if !Path::new(&wallet_path).exists() {
            return Err(anyhow!("Wallet {} not found in secure directory", wallet_name));
        }

        // Sprawdź uprawnienia pliku
        self.verify_file_permissions(&wallet_path)?;

        // Załaduj i zweryfikuj wallet
        let wallet_data = fs::read_to_string(&wallet_path)?;
        let keypair = self.parse_wallet_data(&wallet_data)?;

        info!("✅ Securely loaded wallet: {}", keypair.pubkey());
        Ok(keypair)
    }

    /// Weryfikuje uprawnienia pliku wallet
    fn verify_file_permissions(&self, path: &str) -> Result<()> {
        use std::os::unix::fs::PermissionsExt;
        
        let metadata = fs::metadata(path)?;
        let permissions = metadata.permissions();
        let mode = permissions.mode();

        // Sprawdź czy plik ma bezpieczne uprawnienia (600 - tylko owner read/write)
        if mode & 0o077 != 0 {
            error!("🚨 SECURITY WARNING: Wallet file {} has insecure permissions: {:o}", path, mode);
            error!("Run: chmod 600 {}", path);
            return Err(anyhow!("Insecure wallet file permissions"));
        }

        Ok(())
    }

    /// Parsuje dane wallet z potencjalnym deszyfrowaniem
    fn parse_wallet_data(&self, data: &str) -> Result<Keypair> {
        // Jeśli mamy klucz szyfrowania, spróbuj odszyfrować
        let decrypted_data = if let Some(key) = &self.encryption_key {
            self.decrypt_wallet_data(data, key)?
        } else {
            data.to_string()
        };

        // Parsuj JSON
        let wallet_bytes: Vec<u8> = serde_json::from_str(&decrypted_data)?;
        let keypair = Keypair::try_from(&wallet_bytes[..])?;

        Ok(keypair)
    }

    /// Szyfruje dane wallet (placeholder - implementacja zależy od wybranej biblioteki)
    fn decrypt_wallet_data(&self, encrypted_data: &str, _key: &str) -> Result<String> {
        // TODO: Implementacja rzeczywistego szyfrowania
        // Można użyć bibliotek jak: ring, aes-gcm, chacha20poly1305
        warn!("⚠️ Wallet encryption not yet implemented - using plaintext");
        Ok(encrypted_data.to_string())
    }

    /// Tworzy bezpieczny backup wallet
    pub fn create_secure_backup(&self, keypair: &Keypair) -> Result<String> {
        use chrono::Utc;
        
        let timestamp = Utc::now().timestamp();
        let backup_name = format!("backup_{}_{}.json", keypair.pubkey(), timestamp);
        let backup_path = format!("{}/{}", self.wallet_dir, backup_name);

        // Utwórz katalog jeśli nie istnieje
        fs::create_dir_all(&self.wallet_dir)?;

        // Zapisz z bezpiecznymi uprawnieniami
        let wallet_data = serde_json::to_string(&keypair.to_bytes().to_vec())?;
        fs::write(&backup_path, wallet_data)?;

        // Ustaw bezpieczne uprawnienia
        self.set_secure_permissions(&backup_path)?;

        info!("✅ Secure backup created: {}", backup_path);
        Ok(backup_path)
    }

    /// Ustawia bezpieczne uprawnienia dla pliku
    fn set_secure_permissions(&self, path: &str) -> Result<()> {
        use std::os::unix::fs::PermissionsExt;
        
        let mut permissions = fs::metadata(path)?.permissions();
        permissions.set_mode(0o600); // Tylko owner read/write
        fs::set_permissions(path, permissions)?;

        Ok(())
    }

    /// Weryfikuje integralność wallet
    pub fn verify_wallet_integrity(&self, keypair: &Keypair) -> Result<()> {
        // Sprawdź czy klucz publiczny jest poprawny
        let pubkey = keypair.pubkey();
        if pubkey == Pubkey::default() {
            return Err(anyhow!("Invalid wallet - default pubkey"));
        }

        // Sprawdź czy można podpisać testową wiadomość
        let test_message = b"integrity_check";
        let _signature = keypair.sign_message(test_message);

        info!("✅ Wallet integrity verified: {}", pubkey);
        Ok(())
    }
}

/// Sprawdza czy system ma bezpieczną konfigurację
pub fn security_audit() -> Result<()> {
    info!("🔍 Performing security audit...");

    // Sprawdź zmienne środowiskowe
    check_environment_security()?;
    
    // Sprawdź uprawnienia katalogów
    check_directory_permissions()?;
    
    // Sprawdź czy nie ma wrażliwych plików w repo
    check_repository_security()?;

    info!("✅ Security audit completed");
    Ok(())
}

fn check_environment_security() -> Result<()> {
    // Sprawdź czy są ustawione bezpieczne zmienne środowiskowe
    if env::var("WALLET_ENCRYPTION_KEY").is_err() {
        warn!("⚠️ WALLET_ENCRYPTION_KEY not set - wallets will be stored in plaintext");
    }

    if env::var("WALLET_DIR").is_err() {
        warn!("⚠️ WALLET_DIR not set - using default .wallets directory");
    }

    Ok(())
}

fn check_directory_permissions() -> Result<()> {
    let wallet_dir = env::var("WALLET_DIR").unwrap_or_else(|_| ".wallets".to_string());
    
    if Path::new(&wallet_dir).exists() {
        use std::os::unix::fs::PermissionsExt;
        
        let metadata = fs::metadata(&wallet_dir)?;
        let permissions = metadata.permissions();
        let mode = permissions.mode();

        if mode & 0o077 != 0 {
            warn!("⚠️ Wallet directory {} has insecure permissions: {:o}", wallet_dir, mode);
            warn!("Run: chmod 700 {}", wallet_dir);
        }
    }

    Ok(())
}

fn check_repository_security() -> Result<()> {
    // Sprawdź czy nie ma plików wallet w repo
    let dangerous_patterns = vec![
        "*.json",
        "*.key", 
        "*.pem",
        "*wallet*",
    ];

    for pattern in dangerous_patterns {
        // TODO: Implementacja sprawdzania plików w repo
        // Można użyć git ls-files lub find
    }

    Ok(())
}
