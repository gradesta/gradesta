package main

import (
	"crypto/aes"
	"crypto/cipher"
	"crypto/rand"
	"crypto/sha256"
	"encoding/json"
	"fmt"
	"io"
	"os"
	"path/filepath"
)

// Credential stores Nextcloud access credentials for an identity
type Credential struct {
	NextcloudURL string `json:"nextcloud_url"`
	Username     string `json:"username"`
	AppPassword  string `json:"app_password"`
}

// CredentialStore manages stored credentials
type CredentialStore struct {
	Credentials map[string]*Credential `json:"credentials"`
	configDir   string
}

func getConfigDir() (string, error) {
	home, err := os.UserHomeDir()
	if err != nil {
		return "", err
	}
	return filepath.Join(home, ".config", "gradesta", "nextcloud-file-browser"), nil
}

// loadCredentials loads the credential store from disk
func loadCredentials() (*CredentialStore, error) {
	configDir, err := getConfigDir()
	if err != nil {
		return nil, err
	}

	store := &CredentialStore{
		Credentials: make(map[string]*Credential),
		configDir:   configDir,
	}

	credPath := filepath.Join(configDir, "credentials.enc")
	if _, err := os.Stat(credPath); os.IsNotExist(err) {
		return store, nil
	}

	// Read encrypted file
	encData, err := os.ReadFile(credPath)
	if err != nil {
		return nil, fmt.Errorf("failed to read credentials: %w", err)
	}

	// Load encryption key
	key, err := loadOrCreateKey(configDir)
	if err != nil {
		return nil, fmt.Errorf("failed to load key: %w", err)
	}

	// Decrypt
	data, err := decrypt(encData, key)
	if err != nil {
		return nil, fmt.Errorf("failed to decrypt credentials: %w", err)
	}

	// Parse JSON
	if err := json.Unmarshal(data, store); err != nil {
		return nil, fmt.Errorf("failed to parse credentials: %w", err)
	}

	return store, nil
}

// Save saves the credential store to disk
func (s *CredentialStore) Save() error {
	if err := os.MkdirAll(s.configDir, 0700); err != nil {
		return fmt.Errorf("failed to create config dir: %w", err)
	}

	// Load encryption key
	key, err := loadOrCreateKey(s.configDir)
	if err != nil {
		return fmt.Errorf("failed to load key: %w", err)
	}

	// Serialize to JSON
	data, err := json.MarshalIndent(s, "", "  ")
	if err != nil {
		return fmt.Errorf("failed to serialize credentials: %w", err)
	}

	// Encrypt
	encData, err := encrypt(data, key)
	if err != nil {
		return fmt.Errorf("failed to encrypt credentials: %w", err)
	}

	// Write file
	credPath := filepath.Join(s.configDir, "credentials.enc")
	if err := os.WriteFile(credPath, encData, 0600); err != nil {
		return fmt.Errorf("failed to write credentials: %w", err)
	}

	return nil
}

// Get retrieves credentials for an identity
func (s *CredentialStore) Get(identity string) (*Credential, bool) {
	cred, ok := s.Credentials[identity]
	return cred, ok
}

// Set stores credentials for an identity
func (s *CredentialStore) Set(identity string, cred *Credential) {
	s.Credentials[identity] = cred
}

// loadOrCreateKey loads or generates the encryption key
func loadOrCreateKey(configDir string) ([]byte, error) {
	keyPath := filepath.Join(configDir, "secret.key")

	if data, err := os.ReadFile(keyPath); err == nil && len(data) == 32 {
		return data, nil
	}

	// Generate new key
	key := make([]byte, 32)
	if _, err := rand.Read(key); err != nil {
		return nil, err
	}

	if err := os.MkdirAll(configDir, 0700); err != nil {
		return nil, err
	}

	if err := os.WriteFile(keyPath, key, 0600); err != nil {
		return nil, err
	}

	return key, nil
}

// encrypt encrypts data using AES-256-GCM
func encrypt(plaintext, key []byte) ([]byte, error) {
	// Derive key using SHA-256 to ensure correct length
	hash := sha256.Sum256(key)
	derivedKey := hash[:]

	block, err := aes.NewCipher(derivedKey)
	if err != nil {
		return nil, err
	}

	gcm, err := cipher.NewGCM(block)
	if err != nil {
		return nil, err
	}

	nonce := make([]byte, gcm.NonceSize())
	if _, err := io.ReadFull(rand.Reader, nonce); err != nil {
		return nil, err
	}

	return gcm.Seal(nonce, nonce, plaintext, nil), nil
}

// decrypt decrypts data using AES-256-GCM
func decrypt(ciphertext, key []byte) ([]byte, error) {
	// Derive key using SHA-256 to ensure correct length
	hash := sha256.Sum256(key)
	derivedKey := hash[:]

	block, err := aes.NewCipher(derivedKey)
	if err != nil {
		return nil, err
	}

	gcm, err := cipher.NewGCM(block)
	if err != nil {
		return nil, err
	}

	nonceSize := gcm.NonceSize()
	if len(ciphertext) < nonceSize {
		return nil, fmt.Errorf("ciphertext too short")
	}

	nonce, ciphertext := ciphertext[:nonceSize], ciphertext[nonceSize:]
	return gcm.Open(nil, nonce, ciphertext, nil)
}
