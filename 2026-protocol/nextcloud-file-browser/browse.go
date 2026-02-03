package main

import (
	"encoding/binary"
	"fmt"
	"hash/fnv"
	"log"
	"path"
	"sort"
	"strings"

	"github.com/gorilla/websocket"
)

// entry represents a file or directory in the graph
type entry struct {
	name       string
	fullPath   string
	isDir      bool
	isParent   bool
	entryID    uint64
	contentID  uint64
	northID    uint64
	southID    uint64
	westID     uint64
	eastID     uint64
	labelBytes []byte
	mimeType   string
}

// handleWatchLandmark handles the WatchLandmark message
func handleWatchLandmark(state *ConnectionState, msg []byte) {
	if len(msg) < 9 {
		return
	}

	// Parse: type (1) + action_id (8) + landmark (string)
	// actionID := binary.BigEndian.Uint64(msg[1:9])
	landmark := string(msg[9:])

	log.Printf("Watch landmark: %s", landmark)

	// Parse landmark: nextcloud://{identity}/{path}
	// Expected format: nextcloud://user@server/path/to/dir
	if !strings.HasPrefix(landmark, "nextcloud://") {
		log.Printf("Invalid landmark format: %s", landmark)
		sendLog(state.Conn, 0, 400, 0, "Invalid landmark format")
		return
	}

	// Remove prefix and split
	rest := strings.TrimPrefix(landmark, "nextcloud://")
	parts := strings.SplitN(rest, "/", 2)
	identity := parts[0]
	dirPath := "/"
	if len(parts) > 1 {
		dirPath = "/" + parts[1]
	}

	// Verify identity matches
	if identity != state.Identity {
		log.Printf("Identity mismatch: requested %s, authenticated as %s", identity, state.Identity)
		sendLog(state.Conn, 0, 403, 0, "Cannot access other user's files")
		return
	}

	sendDirectoryListing(state, dirPath)
}

// sendDirectoryListing sends a directory listing as Gradesta graph
func sendDirectoryListing(state *ConnectionState, dirPath string) {
	log.Printf("Listing directory: %s", dirPath)

	// Send context
	landmark := fmt.Sprintf("nextcloud://%s%s", state.Identity, dirPath)
	sendSetContext(state.Conn, 0, landmark)

	// List directory via WebDAV
	entries, err := listWebDAV(state.NextcloudURL, state.Username, state.AppPassword, dirPath)
	if err != nil {
		log.Printf("Failed to list directory: %v", err)
		// Send an error vertex so user sees something
		errorID := hash64("error", state.Identity, dirPath)
		sendSetVertexLabel(state.Conn, 0, errorID, "text/plain", []byte(fmt.Sprintf("Failed to list directory: %v", err)))
		sendSetEdges(state.Conn, 0, errorID, 0, 0, 0, 0, 0, 0, 0)
		return
	}

	// Build entry list
	graphEntries := buildEntries(state, dirPath, entries)

	if len(graphEntries) == 0 {
		// Empty directory - send a placeholder
		emptyID := hash64("empty", state.Identity, dirPath)
		sendSetVertexLabel(state.Conn, 0, emptyID, "text/plain", []byte("(empty directory)"))
		sendSetEdges(state.Conn, 0, emptyID, 0, 0, 0, 0, 0, 0, 0)
		log.Printf("Sent empty directory placeholder for %s", dirPath)
		return
	}

	// Send entries
	for _, e := range graphEntries {
		// Send name label
		sendSetVertexLabel(state.Conn, 0, e.entryID, "text/plain", []byte(e.name))

		// Send content label
		if len(e.labelBytes) > 0 {
			sendSetVertexLabel(state.Conn, 0, e.contentID, e.mimeType, e.labelBytes)
		}

		// Send edges for entry
		sendSetEdges(state.Conn, 0, e.entryID, e.westID, e.eastID, e.northID, e.southID, 0, 0, 0)

		// Send edges for content (if different from entry)
		if e.contentID != 0 && e.contentID != e.entryID {
			sendSetEdges(state.Conn, 0, e.contentID, e.entryID, 0, 0, 0, 0, 0, 0)
		}
	}

	log.Printf("Sent %d entries for %s", len(graphEntries), dirPath)
}

// buildEntries builds the graph entry list from WebDAV entries
func buildEntries(state *ConnectionState, dirPath string, davEntries []DavEntry) []entry {
	var result []entry

	// Add parent directory entry if not at root
	if dirPath != "/" {
		parentPath := path.Dir(strings.TrimSuffix(dirPath, "/"))
		if parentPath == "" {
			parentPath = "/"
		}
		parentLandmark := fmt.Sprintf("nextcloud://%s%s", state.Identity, parentPath)

		result = append(result, entry{
			name:      "..",
			fullPath:  parentPath,
			isDir:     true,
			isParent:  true,
			entryID:   hash64("entry", state.Identity, dirPath, ".."),
			contentID: hash64("dirurl", parentLandmark),
			mimeType:  "text/gradesta-url",
			labelBytes: []byte(parentLandmark),
		})
	}

	// Sort entries alphabetically
	sort.Slice(davEntries, func(i, j int) bool {
		return strings.ToLower(davEntries[i].Name) < strings.ToLower(davEntries[j].Name)
	})

	// Add file/directory entries
	for _, de := range davEntries {
		e := entry{
			name:     de.Name,
			fullPath: de.Path,
			isDir:    de.IsDir,
			entryID:  hash64("entry", state.Identity, dirPath, de.Name),
		}

		if de.IsDir {
			// Directory: link to subdirectory
			subLandmark := fmt.Sprintf("nextcloud://%s%s", state.Identity, de.Path)
			e.contentID = hash64("dirurl", subLandmark)
			e.mimeType = "text/gradesta-url"
			e.labelBytes = []byte(subLandmark)
		} else {
			// File: fetch content (for small files) or provide URL
			if de.Size > 0 && de.Size <= 1024*1024 {
				content, mimeType, err := getWebDAV(state.NextcloudURL, state.Username, state.AppPassword, de.Path)
				if err != nil {
					log.Printf("Failed to fetch %s: %v", de.Path, err)
					e.contentID = hash64("error", de.Path)
					e.mimeType = "text/plain"
					e.labelBytes = []byte(fmt.Sprintf("Error: %v", err))
				} else {
					e.contentID = hash64("file", state.Identity, de.Path)
					e.mimeType = mimeType
					e.labelBytes = content
				}
			} else if de.Size > 1024*1024 {
				// Large file: provide download URL
				downloadURL := fmt.Sprintf("%s/remote.php/dav/files/%s%s",
					state.NextcloudURL, state.Username, de.Path)
				e.contentID = hash64("fileurl", de.Path)
				e.mimeType = "text/x-url"
				e.labelBytes = []byte(downloadURL)
			} else {
				// Unknown size, try to fetch
				content, mimeType, err := getWebDAV(state.NextcloudURL, state.Username, state.AppPassword, de.Path)
				if err != nil {
					e.contentID = hash64("error", de.Path)
					e.mimeType = "text/plain"
					e.labelBytes = []byte(fmt.Sprintf("Error: %v", err))
				} else {
					e.contentID = hash64("file", state.Identity, de.Path)
					e.mimeType = mimeType
					e.labelBytes = content
				}
			}
		}

		result = append(result, e)
	}

	// Establish N/S/E/W links
	for i := range result {
		// North: previous entry
		if i > 0 {
			result[i].northID = result[i-1].entryID
		}
		// South: next entry
		if i < len(result)-1 {
			result[i].southID = result[i+1].entryID
		}
		// East: content
		result[i].eastID = result[i].contentID
		// West: parent directory (only for ".." entry)
		if result[i].isParent {
			result[i].westID = result[i].contentID
		}
	}

	return result
}

// hash64 generates a deterministic 64-bit hash from strings
func hash64(parts ...string) uint64 {
	h := fnv.New64a()
	for _, p := range parts {
		h.Write([]byte(p))
		h.Write([]byte{0})
	}
	return h.Sum64()
}

// Message sending functions

func sendSetContext(conn *websocket.Conn, actionID uint64, landmark string) error {
	msg := make([]byte, 1+8+len(landmark))
	msg[0] = msgServerSetContext
	binary.BigEndian.PutUint64(msg[1:9], actionID)
	copy(msg[9:], landmark)
	err := conn.WriteMessage(websocket.BinaryMessage, msg)
	if err != nil {
		log.Printf("ERROR sending SetContext: %v", err)
	}
	return err
}

func sendSetVertexLabel(conn *websocket.Conn, actionID, vertexID uint64, mimeType string, label []byte) error {
	msg := make([]byte, 1+8+8+len(mimeType)+1+len(label))
	msg[0] = msgServerSetVertexLabel
	binary.BigEndian.PutUint64(msg[1:9], actionID)
	binary.BigEndian.PutUint64(msg[9:17], vertexID)
	copy(msg[17:17+len(mimeType)], mimeType)
	msg[17+len(mimeType)] = 0 // null terminator
	copy(msg[17+len(mimeType)+1:], label)
	err := conn.WriteMessage(websocket.BinaryMessage, msg)
	if err != nil {
		log.Printf("ERROR sending SetVertexLabel (vertex %d): %v", vertexID, err)
	}
	return err
}

func sendSetEdges(conn *websocket.Conn, actionID, vertexID, west, east, north, south, up, down uint64, editMask byte) error {
	msg := make([]byte, 1+8+8+48+1)
	msg[0] = msgServerSetEdges
	binary.BigEndian.PutUint64(msg[1:9], actionID)
	binary.BigEndian.PutUint64(msg[9:17], vertexID)
	binary.BigEndian.PutUint64(msg[17:25], west)
	binary.BigEndian.PutUint64(msg[25:33], east)
	binary.BigEndian.PutUint64(msg[33:41], north)
	binary.BigEndian.PutUint64(msg[41:49], south)
	binary.BigEndian.PutUint64(msg[49:57], up)
	binary.BigEndian.PutUint64(msg[57:65], down)
	msg[65] = editMask
	err := conn.WriteMessage(websocket.BinaryMessage, msg)
	if err != nil {
		log.Printf("ERROR sending SetEdges (vertex %d): %v", vertexID, err)
	}
	return err
}

func sendLog(conn *websocket.Conn, actionID uint64, status int, vertexID uint64, message string) error {
	msg := make([]byte, 1+8+2+8+len(message))
	msg[0] = msgServerLogMessage
	binary.BigEndian.PutUint64(msg[1:9], actionID)
	binary.BigEndian.PutUint16(msg[9:11], uint16(status))
	binary.BigEndian.PutUint64(msg[11:19], vertexID)
	copy(msg[19:], message)
	return conn.WriteMessage(websocket.BinaryMessage, msg)
}
