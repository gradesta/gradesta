package main

import (
	"bytes"
	"crypto/ecdsa"
	"crypto/elliptic"
	"crypto/rand"
	"crypto/sha256"
	"encoding/binary"
	"flag"
	"fmt"
	"io"
	"log"
	"math/big"
	"net/http"
	"net/url"
	"strings"
	"sync"
	"time"

	"github.com/gorilla/websocket"
)

// Protocol message types
const (
	// Server to client
	msgServerSetContext          = 0x01
	msgServerSetVertexLabel      = 0x05
	msgServerSetEdges            = 0x03
	msgServerRequestIdentification = 0x10

	// Client to server
	msgClientWatchLandmark       = 0x81
	msgClientIdentificationResponse = 0x90
	msgClientIdentificationRefused  = 0x91
)

// PendingAuth stores authentication state for a connection
type PendingAuth struct {
	Nonce     [32]byte
	Timestamp int64
	ActionID  uint64
}

var upgrader = websocket.Upgrader{
	CheckOrigin: func(r *http.Request) bool {
		return true
	},
}

var (
	port        = flag.Int("port", 8081, "Server port")
	pendingAuth = make(map[*websocket.Conn]*PendingAuth)
	authMutex   sync.Mutex
)

func main() {
	flag.Parse()

	http.HandleFunc("/ws", handleWebsocket)

	addr := fmt.Sprintf(":%d", *port)
	log.Printf("Hello server starting on %s", addr)
	log.Printf("Connect with: ws://localhost%s/ws", addr)
	log.Fatal(http.ListenAndServe(addr, nil))
}

func handleWebsocket(w http.ResponseWriter, r *http.Request) {
	conn, err := upgrader.Upgrade(w, r, nil)
	if err != nil {
		log.Printf("Upgrade error: %v", err)
		return
	}
	defer conn.Close()

	log.Printf("New connection from %s", conn.RemoteAddr())

	// Send identification request immediately
	if err := sendIdentificationRequest(conn); err != nil {
		log.Printf("Failed to send identification request: %v", err)
		return
	}

	// Handle messages
	for {
		_, msg, err := conn.ReadMessage()
		if err != nil {
			log.Printf("Read error: %v", err)
			return
		}

		if len(msg) < 1 {
			continue
		}

		msgType := msg[0]
		switch msgType {
		case msgClientIdentificationResponse:
			handleIdentificationResponse(conn, msg)
		case msgClientIdentificationRefused:
			log.Printf("Client refused identification")
			// Send a greeting anyway, but without username
			sendGreeting(conn, "anonymous", "")
		case msgClientWatchLandmark:
			// Ignore watch requests until identified
			log.Printf("Ignoring watch landmark (not yet identified)")
		default:
			log.Printf("Unknown message type: 0x%02x", msgType)
		}
	}
}

func sendIdentificationRequest(conn *websocket.Conn) error {
	// Generate nonce
	var nonce [32]byte
	if _, err := rand.Read(nonce[:]); err != nil {
		return fmt.Errorf("failed to generate nonce: %w", err)
	}

	timestamp := time.Now().Unix()
	actionID := uint64(1)

	// Store pending auth
	authMutex.Lock()
	pendingAuth[conn] = &PendingAuth{
		Nonce:     nonce,
		Timestamp: timestamp,
		ActionID:  actionID,
	}
	authMutex.Unlock()

	// Build message: type (1) + action_id (8) + nonce (32) + timestamp (8) + reason (string)
	reason := "Hello server wants to greet you by name"
	msg := make([]byte, 1+8+32+8+len(reason))
	msg[0] = msgServerRequestIdentification
	binary.BigEndian.PutUint64(msg[1:9], actionID)
	copy(msg[9:41], nonce[:])
	binary.BigEndian.PutUint64(msg[41:49], uint64(timestamp))
	copy(msg[49:], reason)

	log.Printf("Sending identification request (nonce: %x...)", nonce[:8])
	return conn.WriteMessage(websocket.BinaryMessage, msg)
}

func handleIdentificationResponse(conn *websocket.Conn, msg []byte) {
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

	// Get pending auth
	authMutex.Lock()
	pending := pendingAuth[conn]
	delete(pendingAuth, conn)
	authMutex.Unlock()

	if pending == nil {
		log.Printf("No pending auth for connection")
		return
	}

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
	claimedUsername := parts[0]
	claimedServer := parts[1]

	// Verify that the claimed server matches the hosting server
	// This prevents someone from hosting a key on evil.com claiming to be user@trusted.com
	if !strings.EqualFold(claimedServer, hostingServer) {
		log.Printf("Identity server mismatch: claimed %s but hosted on %s", claimedServer, hostingServer)
		return
	}

	// Verify signature
	// Signed data: nonce (32) || timestamp (8)
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
	log.Printf("Identified user: %s@%s", claimedUsername, claimedServer)
	sendGreeting(conn, claimedUsername, claimedServer)
}

// PublicKeyData contains the public key and embedded identity
type PublicKeyData struct {
	Identity string
	Key      *ecdsa.PublicKey
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

func sendGreeting(conn *websocket.Conn, username, server string) {
	var greeting string
	if server != "" {
		greeting = fmt.Sprintf("Hello %s@%s!", username, server)
	} else if username != "" {
		greeting = fmt.Sprintf("Hello %s!", username)
	} else {
		greeting = "Hello stranger!"
	}

	// Send SetContext first
	landmark := "hello://greeting"
	contextMsg := make([]byte, 1+8+len(landmark))
	contextMsg[0] = msgServerSetContext
	binary.BigEndian.PutUint64(contextMsg[1:9], 0)
	copy(contextMsg[9:], landmark)
	conn.WriteMessage(websocket.BinaryMessage, contextMsg)

	// Send vertex label with greeting
	vertexID := uint64(1)
	mimeType := "text/plain"
	labelMsg := make([]byte, 1+8+8+len(mimeType)+1+len(greeting))
	labelMsg[0] = msgServerSetVertexLabel
	binary.BigEndian.PutUint64(labelMsg[1:9], 0)
	binary.BigEndian.PutUint64(labelMsg[9:17], vertexID)
	copy(labelMsg[17:17+len(mimeType)], mimeType)
	labelMsg[17+len(mimeType)] = 0 // null terminator
	copy(labelMsg[17+len(mimeType)+1:], greeting)
	conn.WriteMessage(websocket.BinaryMessage, labelMsg)

	// Send edges (no connections)
	edgesMsg := make([]byte, 1+8+8+48+1)
	edgesMsg[0] = msgServerSetEdges
	binary.BigEndian.PutUint64(edgesMsg[1:9], 0)
	binary.BigEndian.PutUint64(edgesMsg[9:17], vertexID)
	// edges are all zeros (no connections)
	// editability bitmask = 0
	conn.WriteMessage(websocket.BinaryMessage, edgesMsg)

	log.Printf("Sent greeting: %s", greeting)
}
