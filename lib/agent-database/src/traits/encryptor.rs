pub trait Encryptor: Send + Sync {
    fn encrypt(&self, plaintext: &str, context: &str)
    -> Result<String, Box<dyn std::error::Error>>;
    fn decrypt(
        &self,
        ciphertext: &str,
        context: &str,
    ) -> Result<String, Box<dyn std::error::Error>>;
}
