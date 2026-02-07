package main

import (
	"flag"
	"fmt"
	"log"
	"net/http"
	"sync"

	"github.com/gorilla/websocket"
)

// Protocol message types
const (
	// Server to client
	msgServerSetContext              = 0x01
	msgServerSetVertexLabel          = 0x05
	msgServerSetEdges                = 0x03
	msgServerRequestIdentification   = 0x10
	msgServerLogMessage              = 0x0F

	// Client to server
	msgClientWatchLandmark           = 0x81
	msgClientIdentificationResponse  = 0x90
	msgClientIdentificationRefused   = 0x91
)

// EdgeUnchanged is the sentinel value meaning "keep existing edge unchanged" when patching edges
const EdgeUnchanged uint64 = 0xFFFFFFFFFFFFFFFF

// Connection states
const (
	stateAwaitingIdentity = "awaiting_identity"
	stateAwaitingAuth     = "awaiting_auth"
	stateBrowsing         = "browsing"
)

var upgrader = websocket.Upgrader{
	CheckOrigin: func(r *http.Request) bool {
		return true
	},
}

var (
	port          = flag.Int("port", 8082, "Server port")
	credStore     *CredentialStore
	credStoreMu   sync.Mutex
)

func main() {
	flag.Parse()

	// Load credential store
	var err error
	credStore, err = loadCredentials()
	if err != nil {
		log.Printf("Warning: Could not load credentials: %v", err)
		credStore = &CredentialStore{Credentials: make(map[string]*Credential)}
	}

	http.HandleFunc("/ws", handleWebsocket)

	addr := fmt.Sprintf(":%d", *port)
	log.Printf("Nextcloud file browser starting on %s", addr)
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

	// Create connection state
	state := &ConnectionState{
		Conn:  conn,
		State: stateAwaitingIdentity,
	}

	// Send identification request
	pending, err := sendIdentificationRequest(conn)
	if err != nil {
		log.Printf("Failed to send identification request: %v", err)
		return
	}
	state.PendingAuth = pending

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

		handleClientMessage(state, msg)
	}
}

func handleClientMessage(state *ConnectionState, msg []byte) {
	msgType := msg[0]

	switch msgType {
	case msgClientIdentificationResponse:
		handleIdentificationResponse(state, msg)

	case msgClientIdentificationRefused:
		log.Printf("Client refused identification")
		sendLog(state.Conn, 0, 403, 0, "Identification required to browse Nextcloud files")

	case msgClientWatchLandmark:
		if state.State != stateBrowsing {
			log.Printf("Ignoring watch landmark (not yet authenticated)")
			sendLog(state.Conn, 0, 403, 0, "Please complete authentication first")
			return
		}
		handleWatchLandmark(state, msg)

	default:
		log.Printf("Unknown message type: 0x%02x", msgType)
	}
}
