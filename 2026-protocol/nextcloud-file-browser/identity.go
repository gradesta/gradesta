package main

import (
	"bytes"
	"crypto/ecdsa"
	"crypto/elliptic"
	"crypto/rand"
	"crypto/sha256"
	"encoding/binary"
	"fmt"
	"io"
	"log"
	"math/big"
	"net/http"
	"net/url"
	"strings"
	"time"

	"github.com/gorilla/websocket"
)

// PendingAuth stores authentication state for a connection
type PendingAuth struct {
	Nonce     [32]byte
	Timestamp int64
	ActionID  uint64
}

// ConnectionState tracks the state of a WebSocket connection
type ConnectionState struct {
	Conn         *websocket.Conn
	Identity     string // "username@server"
	NextcloudURL string // "https://server"
	Username     string
	AppPassword  string
	State        string // awaiting_identity, awaiting_auth, browsing
	PendingAuth  *PendingAuth
	LoginPoll    *LoginPollState
}

// LoginPollState tracks Login Flow v2 polling
type LoginPollState struct {
	PollEndpoint string
	PollToken    string
	StopChan     chan struct{}
}

// PublicKeyData contains the public key and embedded identity
type PublicKeyData struct {
	Identity string
	Key      *ecdsa.PublicKey
}

func sendIdentificationRequest(conn *websocket.Conn) (*PendingAuth, error) {
	// Generate nonce
	var nonce [32]byte
	if _, err := rand.Read(nonce[:]); err != nil {
		return nil, fmt.Errorf("failed to generate nonce: %w", err)
	}

	timestamp := time.Now().Unix()
	actionID := uint64(1)

	pending := &PendingAuth{
		Nonce:     nonce,
		Timestamp: timestamp,
		ActionID:  actionID,
	}

	// Build message: type (1) + action_id (8) + nonce (32) + timestamp (8) + reason (string)
	reason := "Nextcloud file browser needs to verify your identity"
	msg := make([]byte, 1+8+32+8+len(reason))
	msg[0] = msgServerRequestIdentification
	binary.BigEndian.PutUint64(msg[1:9], actionID)
	copy(msg[9:41], nonce[:])
	binary.BigEndian.PutUint64(msg[41:49], uint64(timestamp))
	copy(msg[49:], reason)

	log.Printf("Sending identification request (nonce: %x...)", nonce[:8])
	return pending, conn.WriteMessage(websocket.BinaryMessage, msg)
}

func handleIdentificationResponse(state *ConnectionState, msg []byte) {
	// Parse: type (1) + action_id (8) + identity_url (null-terminated) + signature (64)
	if len(msg) < 1+8+1+64 {
		log.Printf("Identification response too short")
		return
	}

	actionID := binary.BigEndian.Uint64(msg[1:9])

	// Find null terminator for identity URL
	urlEnd := -1
	for i := 9; i < len(msg)-64; i++ {
		if msg[i] == 0 {
			urlEnd = i
			break
		}
	}
	if urlEnd == -1 {
		log.Printf("No null terminator found in identity URL")
		return
	}

	identityURL := string(msg[9:urlEnd])
	signature := msg[urlEnd+1:]

	if len(signature) != 64 {
		log.Printf("Invalid signature length: %d (expected 64)", len(signature))
		return
	}

	log.Printf("Received identification response: %s", identityURL)

	pending := state.PendingAuth
	if pending == nil {
		log.Printf("No pending auth for connection")
		return
	}
	state.PendingAuth = nil

	if actionID != pending.ActionID {
		log.Printf("Action ID mismatch: got %d, expected %d", actionID, pending.ActionID)
		return
	}

	// Check timestamp is within ±5 minutes
	now := time.Now().Unix()
	if pending.Timestamp < now-300 || pending.Timestamp > now+300 {
		log.Printf("Timestamp out of range: %d (now: %d)", pending.Timestamp, now)
		return
	}

	// Parse the identity URL to get the hosting server
	parsedURL, err := url.Parse(identityURL)
	if err != nil {
		log.Printf("Failed to parse identity URL: %v", err)
		return
	}
	hostingServer := parsedURL.Host

	// Fetch public key from identity URL
	pubKeyData, err := fetchPublicKey(identityURL)
	if err != nil {
		log.Printf("Failed to fetch public key: %v", err)
		return
	}

	log.Printf("Fetched public key with identity: %s", pubKeyData.Identity)

	// Parse identity from public key file (format: username@server)
	identity := pubKeyData.Identity
	parts := strings.SplitN(identity, "@", 2)
	if len(parts) != 2 {
		log.Printf("Invalid identity format: %s", identity)
		return
	}
	username := parts[0]
	server := parts[1]

	// Verify that the claimed server matches the hosting server
	if !strings.EqualFold(server, hostingServer) {
		log.Printf("Identity server mismatch: claimed %s but hosted on %s", server, hostingServer)
		return
	}

	// Verify signature
	signedData := make([]byte, 40)
	copy(signedData[:32], pending.Nonce[:])
	binary.BigEndian.PutUint64(signedData[32:], uint64(pending.Timestamp))

	hash := sha256.Sum256(signedData)

	r := new(big.Int).SetBytes(signature[:32])
	s := new(big.Int).SetBytes(signature[32:64])

	if !ecdsa.Verify(pubKeyData.Key, hash[:], r, s) {
		log.Printf("Signature verification failed")
		return
	}

	log.Printf("Signature verified successfully!")
	log.Printf("Identified user: %s@%s", username, server)

	// Store identity info
	state.Identity = identity
	state.Username = username
	state.NextcloudURL = fmt.Sprintf("https://%s", server)

	// Check if we have stored credentials for this identity
	credStoreMu.Lock()
	cred, exists := credStore.Get(identity)
	credStoreMu.Unlock()

	if exists {
		log.Printf("Found stored credentials for %s", identity)
		state.Username = cred.Username
		state.AppPassword = cred.AppPassword
		state.NextcloudURL = cred.NextcloudURL
		state.State = stateBrowsing
		// Send root directory listing
		sendDirectoryListing(state, "/")
	} else {
		log.Printf("No credentials for %s, starting Nextcloud auth flow", identity)
		startNextcloudAuth(state)
	}
}

func fetchPublicKey(identityURL string) (*PublicKeyData, error) {
	// Add /download to get the raw file
	downloadURL := identityURL + "/download"

	resp, err := http.Get(downloadURL)
	if err != nil {
		return nil, fmt.Errorf("failed to fetch: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != 200 {
		return nil, fmt.Errorf("HTTP %d", resp.StatusCode)
	}

	// Read public key file
	// Format: identity_string (null-terminated) + public key (65 bytes: 0x04 || X || Y)
	keyData, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil, fmt.Errorf("failed to read body: %w", err)
	}

	// Find null terminator to split identity from key
	nullIdx := bytes.IndexByte(keyData, 0)
	if nullIdx == -1 {
		return nil, fmt.Errorf("no identity found in public key file")
	}

	identity := string(keyData[:nullIdx])
	keyBytes := keyData[nullIdx+1:]

	// Parse uncompressed point format (0x04 || X || Y)
	if len(keyBytes) != 65 || keyBytes[0] != 0x04 {
		return nil, fmt.Errorf("invalid public key format (len=%d)", len(keyBytes))
	}

	x := new(big.Int).SetBytes(keyBytes[1:33])
	y := new(big.Int).SetBytes(keyBytes[33:65])

	pubKey := &ecdsa.PublicKey{
		Curve: elliptic.P256(),
		X:     x,
		Y:     y,
	}

	return &PublicKeyData{
		Identity: identity,
		Key:      pubKey,
	}, nil
}
