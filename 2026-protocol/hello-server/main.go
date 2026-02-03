package main

import (
	"crypto/ecdsa"
	"crypto/elliptic"
	"crypto/rand"
	"crypto/sha256"
	"encoding/binary"
	"encoding/json"
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

	// Fetch public key from identity URL
	pubKey, err := fetchPublicKey(identityURL)
	if err != nil {
		log.Printf("Failed to fetch public key: %v", err)
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

	if !ecdsa.Verify(pubKey, hash[:], r, s) {
		log.Printf("Signature verification failed")
		return
	}

	log.Printf("Signature verified successfully!")

	// Resolve username from Nextcloud
	username, server, err := resolveIdentity(identityURL)
	if err != nil {
		log.Printf("Failed to resolve identity: %v", err)
		// Use URL as fallback
		sendGreeting(conn, identityURL, "")
		return
	}

	log.Printf("Identified user: %s@%s", username, server)
	sendGreeting(conn, username, server)
}

func fetchPublicKey(identityURL string) (*ecdsa.PublicKey, error) {
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

	// Read public key (expected: 65 bytes uncompressed P-256 point)
	keyData, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil, fmt.Errorf("failed to read body: %w", err)
	}

	// Parse uncompressed point format (0x04 || X || Y)
	if len(keyData) != 65 || keyData[0] != 0x04 {
		return nil, fmt.Errorf("invalid public key format (len=%d)", len(keyData))
	}

	x := new(big.Int).SetBytes(keyData[1:33])
	y := new(big.Int).SetBytes(keyData[33:65])

	pubKey := &ecdsa.PublicKey{
		Curve: elliptic.P256(),
		X:     x,
		Y:     y,
	}

	return pubKey, nil
}

func resolveIdentity(identityURL string) (username string, server string, err error) {
	// Parse the share URL to extract server and share token
	// Format: https://nextcloud.example.com/s/{share-token}
	parsed, err := url.Parse(identityURL)
	if err != nil {
		return "", "", fmt.Errorf("failed to parse URL: %w", err)
	}

	server = parsed.Host

	// Extract share token from path
	parts := strings.Split(strings.Trim(parsed.Path, "/"), "/")
	if len(parts) < 2 || parts[0] != "s" {
		return "", "", fmt.Errorf("invalid share URL format")
	}
	shareToken := parts[1]

	// Query Nextcloud OCS API for share info
	// GET /ocs/v2.php/apps/files_sharing/api/v1/shares/{token}
	ocsURL := fmt.Sprintf("%s://%s/ocs/v2.php/apps/files_sharing/api/v1/shares/%s",
		parsed.Scheme, parsed.Host, shareToken)

	req, err := http.NewRequest("GET", ocsURL, nil)
	if err != nil {
		return "", "", fmt.Errorf("failed to create request: %w", err)
	}
	req.Header.Set("OCS-APIRequest", "true")
	req.Header.Set("Accept", "application/json")

	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		return "", "", fmt.Errorf("OCS request failed: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != 200 {
		return "", "", fmt.Errorf("OCS HTTP %d", resp.StatusCode)
	}

	// Parse OCS response
	var ocsResp struct {
		OCS struct {
			Data []struct {
				UIDOwner         string `json:"uid_owner"`
				DisplaynameOwner string `json:"displayname_owner"`
			} `json:"data"`
		} `json:"ocs"`
	}

	if err := json.NewDecoder(resp.Body).Decode(&ocsResp); err != nil {
		return "", "", fmt.Errorf("failed to decode OCS response: %w", err)
	}

	if len(ocsResp.OCS.Data) == 0 {
		return "", "", fmt.Errorf("no share data returned")
	}

	username = ocsResp.OCS.Data[0].UIDOwner
	return username, server, nil
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
