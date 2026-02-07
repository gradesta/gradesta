package main

import (
	"bytes"
	"encoding/binary"
	"errors"
	"fmt"
	"hash/fnv"
	"log"
	"net/http"
	"os"
	"os/exec"
	"path/filepath"
	"sort"
	"strings"

	"github.com/gorilla/websocket"
	"github.com/spf13/pflag"
)

const (
	msgClientWatchLandmark     = 0x81
	msgClientStopWatchLandmark = 0x82
	msgClientSetEdges          = 0x83
	msgClientClickVertex       = 0x84
	msgClientSetVertexLabel    = 0x85

	msgServerSetContext     = 0x01
	msgServerSetEdges       = 0x03
	msgServerSetVertexLabel = 0x05
	msgServerLogMessage     = 0x0F
)

// EdgeUnchanged is the sentinel value meaning "keep existing edge unchanged" when patching edges
const EdgeUnchanged uint64 = 0xFFFFFFFFFFFFFFFF

const editabilityMaskNone byte = 0x00

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

var upgrader = websocket.Upgrader{
	CheckOrigin: func(r *http.Request) bool {
		return true
	},
}

func main() {
	port := pflag.Int("port", 8080, "listen port")
	pflag.Parse()

	http.HandleFunc("/ws", handleWebsocket)
	addr := fmt.Sprintf(":%d", *port)
	log.Printf("gradesta file-browser listening on %s", addr)
	if err := http.ListenAndServe(addr, nil); err != nil {
		log.Fatal(err)
	}
}

func handleWebsocket(w http.ResponseWriter, r *http.Request) {
	conn, err := upgrader.Upgrade(w, r, nil)
	if err != nil {
		log.Printf("upgrade failed: %v", err)
		return
	}
	defer conn.Close()

	for {
		msgType, data, err := conn.ReadMessage()
		if err != nil {
			if websocket.IsCloseError(err, websocket.CloseNormalClosure, websocket.CloseGoingAway) {
				return
			}
			log.Printf("read error: %v", err)
			return
		}
		if msgType != websocket.BinaryMessage {
			continue
		}
		if err := handleClientMessage(conn, data); err != nil {
			log.Printf("handle message error: %v", err)
		}
	}
}

func handleClientMessage(conn *websocket.Conn, data []byte) error {
	if len(data) < 1 {
		return errors.New("message too short")
	}
	msgType := data[0]
	payload := data[1:]

	switch msgType {
	case msgClientWatchLandmark:
		actionID, rest, err := readU64(payload)
		if err != nil {
			_ = sendLog(conn, 0, uint32(http.StatusBadRequest), 0, "invalid watch message: missing action id")
			return err
		}
		uri := string(rest)
		log.Printf("RECV WatchLandmark action=%d uri=%q", actionID, uri)
		return handleWatchLandmark(conn, actionID, uri)
	case msgClientStopWatchLandmark:
		actionID, rest, _ := readU64(payload)
		uri := string(rest)
		log.Printf("RECV StopWatchLandmark action=%d uri=%q", actionID, uri)
		return nil
	case msgClientSetEdges:
		log.Printf("RECV SetEdges (ignored)")
		return nil
	case msgClientClickVertex:
		log.Printf("RECV ClickVertex (ignored)")
		return nil
	case msgClientSetVertexLabel:
		log.Printf("RECV SetVertexLabel (ignored)")
		return nil
	default:
		log.Printf("RECV Unknown message type: 0x%02x", msgType)
		return fmt.Errorf("unknown message type: 0x%02x", msgType)
	}
}

func handleWatchLandmark(conn *websocket.Conn, actionID uint64, uri string) error {
	dirPath, err := parseLandmarkURI(uri)
	if err != nil {
		_ = sendLog(conn, actionID, uint32(http.StatusBadRequest), 0, err.Error())
		return nil // Don't crash, just log
	}

	info, err := os.Stat(dirPath)
	if err != nil {
		if os.IsNotExist(err) {
			_ = sendLog(conn, actionID, uint32(http.StatusNotFound), 0, fmt.Sprintf("not found: %s", dirPath))
		} else if os.IsPermission(err) {
			_ = sendLog(conn, actionID, uint32(http.StatusForbidden), 0, fmt.Sprintf("permission denied: %s", dirPath))
		} else {
			_ = sendLog(conn, actionID, uint32(http.StatusInternalServerError), 0, err.Error())
		}
		return nil // Don't crash, just log
	}
	if !info.IsDir() {
		_ = sendLog(conn, actionID, uint32(http.StatusBadRequest), 0, fmt.Sprintf("not a directory: %s", dirPath))
		return nil // Don't crash, just log
	}

	dirURI := normalizeDirURI(dirPath)
	if err := sendSetContext(conn, actionID, dirURI); err != nil {
		return err
	}

	entries, err := buildEntries(dirPath)
	if err != nil {
		if os.IsPermission(err) {
			_ = sendLog(conn, actionID, uint32(http.StatusForbidden), 0, fmt.Sprintf("permission denied: %s", dirPath))
		} else {
			_ = sendLog(conn, actionID, uint32(http.StatusInternalServerError), 0, err.Error())
		}
		return nil // Don't crash, just log
	}

	for i := range entries {
		entry := &entries[i]
		if err := sendSetVertexLabel(conn, actionID, entry.entryID, "text/plain", []byte(entry.name)); err != nil {
			return err
		}
		if entry.contentID != 0 {
			if err := sendSetVertexLabel(conn, actionID, entry.contentID, entry.mimeType, entry.labelBytes); err != nil {
				return err
			}
			// Content's WEST edge points back to entry (so user can navigate back)
			// Exception: for ".." entry, the content IS the west link, so don't point back
			contentWestID := uint64(0)
			if !entry.isParent {
				contentWestID = entry.entryID
			}
			if err := sendSetEdges(conn, actionID, entry.contentID, contentWestID, 0, 0, 0, 0, 0, editabilityMaskNone); err != nil {
				return err
			}
		}
		if err := sendSetEdges(
			conn,
			actionID,
			entry.entryID,
			entry.westID,
			entry.eastID,
			entry.northID,
			entry.southID,
			0,
			0,
			editabilityMaskNone,
		); err != nil {
			return err
		}
	}

	return nil
}

func buildEntries(dirPath string) ([]entry, error) {
	dirPath = filepath.Clean(dirPath)
	parentPath := filepath.Dir(dirPath)
	if dirPath == "/" {
		parentPath = "/"
	}
	parentURI := normalizeDirURI(parentPath)

	entries := []entry{
		{
			name:      "..",
			fullPath:  parentPath,
			isDir:     true,
			isParent:  true,
			entryID:   hash64("entry", dirPath, ".."),
			contentID: hash64("dirurl", parentURI),
			mimeType:  "text/gradesta-url",
			labelBytes: []byte(parentURI),
		},
	}

	dirEntries, err := os.ReadDir(dirPath)
	if err != nil {
		return nil, err
	}

	for _, de := range dirEntries {
		name := de.Name()
		fullPath := filepath.Join(dirPath, name)
		isDir := de.IsDir()
		contentID := uint64(0)
		mimeType := ""
		var labelBytes []byte

		if isDir {
			// Check if we can access the directory
			_, err := os.Stat(fullPath)
			if err != nil {
				// Can't access - still show the entry but with error message as content
				contentID = hash64("error", fullPath)
				if os.IsPermission(err) {
					mimeType = "text/plain"
					labelBytes = []byte("Permission denied")
				} else {
					mimeType = "text/plain"
					labelBytes = []byte(err.Error())
				}
			} else {
				dirURI := normalizeDirURI(fullPath)
				contentID = hash64("dirurl", dirURI)
				mimeType = "text/gradesta-url"
				labelBytes = []byte(dirURI)
			}
		} else {
			contentID = hash64("file", fullPath)
			mimeType = detectMimeType(fullPath)
			
			// Check file size first - don't send huge files inline
			const maxInlineSize = 1 * 1024 * 1024 // 1MB limit
			fileInfo, err := os.Stat(fullPath)
			if err != nil {
				if os.IsPermission(err) {
					mimeType = "text/plain"
					labelBytes = []byte("Permission denied")
				} else {
					mimeType = "text/plain"
					labelBytes = []byte(err.Error())
				}
			} else if fileInfo.Size() > maxInlineSize {
				// File too large - use text/x-url so client can open externally
				mimeType = "text/x-url"
				labelBytes = []byte("file://" + fullPath)
			} else {
				fileBytes, err := os.ReadFile(fullPath)
				if err != nil {
					// Can't read file - show error message
					if os.IsPermission(err) {
						mimeType = "text/plain"
						labelBytes = []byte("Permission denied")
					} else {
						mimeType = "text/plain"
						labelBytes = []byte(err.Error())
					}
				} else {
					labelBytes = fileBytes
				}
			}
		}

		entries = append(entries, entry{
			name:       name,
			fullPath:   fullPath,
			isDir:      isDir,
			entryID:    hash64("entry", dirPath, name),
			contentID:  contentID,
			mimeType:   mimeType,
			labelBytes: labelBytes,
		})
	}

	sort.Slice(entries[1:], func(i, j int) bool {
		return strings.ToLower(entries[i+1].name) < strings.ToLower(entries[j+1].name)
	})

	for i := range entries {
		if i > 0 {
			entries[i].northID = entries[i-1].entryID
		}
		if i+1 < len(entries) {
			entries[i].southID = entries[i+1].entryID
		}

		if entries[i].isParent {
			entries[i].westID = entries[i].contentID
		} else if entries[i].contentID != 0 {
			entries[i].eastID = entries[i].contentID
		}
	}

	return entries, nil
}

func parseLandmarkURI(uri string) (string, error) {
	trimmed := strings.TrimSpace(uri)
	if trimmed == "" {
		return "", errors.New("empty landmark uri")
	}
	if !strings.HasPrefix(trimmed, "/") {
		return "", fmt.Errorf("landmark uri must be an absolute path: %s", trimmed)
	}
	cleaned := filepath.Clean(trimmed)
	return cleaned, nil
}

func normalizeDirURI(dirPath string) string {
	dirPath = filepath.Clean(dirPath)
	if dirPath == "/" {
		return "/"
	}
	return dirPath + "/"
}

func detectMimeType(path string) string {
	// First try the file command
	cmd := exec.Command("file", "--mime-type", "-b", path)
	output, err := cmd.Output()
	if err == nil {
		mime := strings.TrimSpace(string(output))
		if mime != "" && mime != "application/octet-stream" {
			return mime
		}
	}

	// Fallback to extension-based detection
	ext := strings.ToLower(filepath.Ext(path))
	switch ext {
	// Images
	case ".jpg", ".jpeg":
		return "image/jpeg"
	case ".png":
		return "image/png"
	case ".gif":
		return "image/gif"
	case ".webp":
		return "image/webp"
	case ".bmp":
		return "image/bmp"
	case ".svg":
		return "image/svg+xml"
	case ".ico":
		return "image/x-icon"
	// Text
	case ".txt":
		return "text/plain"
	case ".html", ".htm":
		return "text/html"
	case ".css":
		return "text/css"
	case ".js":
		return "text/javascript"
	case ".json":
		return "application/json"
	case ".xml":
		return "text/xml"
	case ".md":
		return "text/markdown"
	case ".csv":
		return "text/csv"
	// Code
	case ".go":
		return "text/x-go"
	case ".rs":
		return "text/x-rust"
	case ".py":
		return "text/x-python"
	case ".rb":
		return "text/x-ruby"
	case ".java":
		return "text/x-java"
	case ".c", ".h":
		return "text/x-c"
	case ".cpp", ".hpp", ".cc":
		return "text/x-c++"
	case ".sh":
		return "text/x-shellscript"
	case ".yaml", ".yml":
		return "text/yaml"
	case ".toml":
		return "text/toml"
	// Documents
	case ".pdf":
		return "application/pdf"
	// Audio
	case ".mp3":
		return "audio/mpeg"
	case ".wav":
		return "audio/wav"
	case ".ogg":
		return "audio/ogg"
	case ".flac":
		return "audio/flac"
	// Video
	case ".mp4":
		return "video/mp4"
	case ".webm":
		return "video/webm"
	case ".mkv":
		return "video/x-matroska"
	case ".avi":
		return "video/x-msvideo"
	// Archives
	case ".zip":
		return "application/zip"
	case ".tar":
		return "application/x-tar"
	case ".gz":
		return "application/gzip"
	default:
		return "application/octet-stream"
	}
}

func hash64(parts ...string) uint64 {
	hasher := fnv.New64a()
	for _, part := range parts {
		_, _ = hasher.Write([]byte(part))
		_, _ = hasher.Write([]byte{0})
	}
	return hasher.Sum64()
}

func readU64(data []byte) (uint64, []byte, error) {
	if len(data) < 8 {
		return 0, nil, errors.New("not enough bytes for uint64")
	}
	return binary.BigEndian.Uint64(data[:8]), data[8:], nil
}

func writeU64(buf *bytes.Buffer, value uint64) {
	var tmp [8]byte
	binary.BigEndian.PutUint64(tmp[:], value)
	buf.Write(tmp[:])
}

func writeU32(buf *bytes.Buffer, value uint32) {
	var tmp [4]byte
	binary.BigEndian.PutUint32(tmp[:], value)
	buf.Write(tmp[:])
}

func sendSetContext(conn *websocket.Conn, actionID uint64, uri string) error {
	log.Printf("SEND SetContext action=%d uri=%q", actionID, uri)
	var buf bytes.Buffer
	buf.WriteByte(msgServerSetContext)
	writeU64(&buf, actionID)
	buf.WriteString(uri)
	return conn.WriteMessage(websocket.BinaryMessage, buf.Bytes())
}

func sendSetVertexLabel(conn *websocket.Conn, actionID, vertexID uint64, mimeType string, label []byte) error {
	labelPreview := string(label)
	if len(labelPreview) > 40 {
		labelPreview = labelPreview[:40] + "..."
	}
	log.Printf("SEND SetVertexLabel action=%d vertex=%d mime=%q label=%q (%d bytes)", actionID, vertexID, mimeType, labelPreview, len(label))
	var buf bytes.Buffer
	buf.WriteByte(msgServerSetVertexLabel)
	writeU64(&buf, actionID)
	writeU64(&buf, vertexID)
	buf.WriteString(mimeType)
	buf.WriteByte(0)
	buf.Write(label)
	return conn.WriteMessage(websocket.BinaryMessage, buf.Bytes())
}

func sendSetEdges(
	conn *websocket.Conn,
	actionID uint64,
	vertexID uint64,
	westID uint64,
	eastID uint64,
	northID uint64,
	southID uint64,
	upID uint64,
	downID uint64,
	editabilityMask byte,
) error {
	log.Printf("SEND SetEdges action=%d vertex=%d W=%d E=%d N=%d S=%d U=%d D=%d", actionID, vertexID, westID, eastID, northID, southID, upID, downID)
	var buf bytes.Buffer
	buf.WriteByte(msgServerSetEdges)
	writeU64(&buf, actionID)
	writeU64(&buf, vertexID)
	writeU64(&buf, westID)
	writeU64(&buf, eastID)
	writeU64(&buf, northID)
	writeU64(&buf, southID)
	writeU64(&buf, upID)
	writeU64(&buf, downID)
	buf.WriteByte(editabilityMask)
	return conn.WriteMessage(websocket.BinaryMessage, buf.Bytes())
}

func sendLog(conn *websocket.Conn, actionID uint64, status uint32, vertexID uint64, message string) error {
	log.Printf("SEND Log action=%d status=%d vertex=%d msg=%q", actionID, status, vertexID, message)
	var buf bytes.Buffer
	buf.WriteByte(msgServerLogMessage)
	writeU64(&buf, actionID)
	writeU32(&buf, status)
	writeU64(&buf, vertexID)
	buf.WriteString(message)
	return conn.WriteMessage(websocket.BinaryMessage, buf.Bytes())
}
