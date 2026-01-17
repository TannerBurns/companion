use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use keyring::Entry;
use rand::RngCore;
use thiserror::Error;

const SERVICE_NAME: &str = "companion-app";
const MASTER_KEY_NAME: &str = "master-encryption-key";
const NONCE_SIZE: usize = 12;

#[derive(Error, Debug)]
pub enum CryptoError {
    #[error("Keyring error: {0}")]
    Keyring(#[from] keyring::Error),
    
    #[error("Encryption error")]
    Encryption,
    
    #[error("Decryption error")]
    Decryption,
    
    #[error("Invalid key length")]
    InvalidKeyLength,
    
    #[error("Base64 decode error: {0}")]
    Base64(#[from] base64::DecodeError),
}

#[derive(Clone)]
pub struct CryptoService {
    cipher: Aes256Gcm,
}

impl CryptoService {
    pub fn new() -> Result<Self, CryptoError> {
        let master_key = Self::get_or_create_master_key()?;
        let cipher = Aes256Gcm::new_from_slice(&master_key)
            .map_err(|_| CryptoError::InvalidKeyLength)?;
        
        Ok(Self { cipher })
    }
    
    fn get_or_create_master_key() -> Result<[u8; 32], CryptoError> {
        let entry = Entry::new(SERVICE_NAME, MASTER_KEY_NAME)?;
        
        match entry.get_password() {
            Ok(key_b64) => {
                let key_bytes = BASE64.decode(&key_b64)?;
                
                let mut key = [0u8; 32];
                if key_bytes.len() != 32 {
                    return Err(CryptoError::InvalidKeyLength);
                }
                key.copy_from_slice(&key_bytes);
                
                tracing::info!("Retrieved master key from keychain");
                Ok(key)
            }
            Err(keyring::Error::NoEntry) => {
                let mut key = [0u8; 32];
                OsRng.fill_bytes(&mut key);
                
                let key_b64 = BASE64.encode(key);
                entry.set_password(&key_b64)?;
                
                tracing::info!("Generated and stored new master key in keychain");
                Ok(key)
            }
            Err(e) => Err(CryptoError::Keyring(e)),
        }
    }
    
    pub fn encrypt(&self, plaintext: &[u8]) -> Result<String, CryptoError> {
        let mut nonce_bytes = [0u8; NONCE_SIZE];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        
        let ciphertext = self.cipher
            .encrypt(nonce, plaintext)
            .map_err(|_| CryptoError::Encryption)?;
        
        let mut result = Vec::with_capacity(NONCE_SIZE + ciphertext.len());
        result.extend_from_slice(&nonce_bytes);
        result.extend_from_slice(&ciphertext);
        
        Ok(BASE64.encode(&result))
    }
    
    pub fn decrypt(&self, ciphertext_b64: &str) -> Result<Vec<u8>, CryptoError> {
        let data = BASE64.decode(ciphertext_b64)?;
        
        if data.len() < NONCE_SIZE {
            return Err(CryptoError::Decryption);
        }
        
        let (nonce_bytes, ciphertext) = data.split_at(NONCE_SIZE);
        let nonce = Nonce::from_slice(nonce_bytes);
        
        self.cipher
            .decrypt(nonce, ciphertext)
            .map_err(|_| CryptoError::Decryption)
    }
    
    pub fn encrypt_string(&self, plaintext: &str) -> Result<String, CryptoError> {
        self.encrypt(plaintext.as_bytes())
    }
    
    pub fn decrypt_string(&self, ciphertext_b64: &str) -> Result<String, CryptoError> {
        let bytes = self.decrypt(ciphertext_b64)?;
        String::from_utf8(bytes).map_err(|_| CryptoError::Decryption)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let crypto = CryptoService::new().unwrap();
        let plaintext = "Hello, World!";
        
        let encrypted = crypto.encrypt_string(plaintext).unwrap();
        let decrypted = crypto.decrypt_string(&encrypted).unwrap();
        
        assert_eq!(plaintext, decrypted);
    }
}
