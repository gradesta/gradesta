package main

import (
	"encoding/json"
	"encoding/xml"
	"fmt"
	"io"
	"log"
	"net/http"
	"path"
	"strings"
	"time"
)

// LoginFlowInit is the response from POST /index.php/login/v2
type LoginFlowInit struct {
	Poll  LoginFlowPoll `json:"poll"`
	Login string        `json:"login"`
}

type LoginFlowPoll struct {
	Token    string `json:"token"`
	Endpoint string `json:"endpoint"`
}

// LoginFlowResult is the response when polling completes
type LoginFlowResult struct {
	Server      string `json:"server"`
	LoginName   string `json:"loginName"`
	AppPassword string `json:"appPassword"`
}

// initiateLoginFlow starts the Nextcloud Login Flow v2
func initiateLoginFlow(nextcloudURL string) (loginURL, pollEndpoint, pollToken string, err error) {
	endpoint := fmt.Sprintf("%s/index.php/login/v2", strings.TrimSuffix(nextcloudURL, "/"))

	resp, err := http.Post(endpoint, "application/x-www-form-urlencoded", nil)
	if err != nil {
		return "", "", "", fmt.Errorf("failed to initiate login flow: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != 200 {
		return "", "", "", fmt.Errorf("login flow returned status %d", resp.StatusCode)
	}

	var result LoginFlowInit
	if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
		return "", "", "", fmt.Errorf("failed to decode login flow response: %w", err)
	}

	return result.Login, result.Poll.Endpoint, result.Poll.Token, nil
}

// pollLoginCompletion checks if the user has completed login
func pollLoginCompletion(pollEndpoint, pollToken string) (username, appPassword string, done bool, err error) {
	client := &http.Client{Timeout: 5 * time.Second}

	req, err := http.NewRequest("POST", pollEndpoint, strings.NewReader("token="+pollToken))
	if err != nil {
		return "", "", false, err
	}
	req.Header.Set("Content-Type", "application/x-www-form-urlencoded")

	resp, err := client.Do(req)
	if err != nil {
		return "", "", false, nil // Network error, will retry
	}
	defer resp.Body.Close()

	if resp.StatusCode == 404 {
		// Not yet completed
		return "", "", false, nil
	}

	if resp.StatusCode != 200 {
		return "", "", false, fmt.Errorf("poll returned status %d", resp.StatusCode)
	}

	var result LoginFlowResult
	if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
		return "", "", false, fmt.Errorf("failed to decode poll response: %w", err)
	}

	return result.LoginName, result.AppPassword, true, nil
}

// startNextcloudAuth initiates the auth flow and starts polling
func startNextcloudAuth(state *ConnectionState) {
	loginURL, pollEndpoint, pollToken, err := initiateLoginFlow(state.NextcloudURL)
	if err != nil {
		log.Printf("Failed to initiate login flow: %v", err)
		sendLog(state.Conn, 0, 500, 0, fmt.Sprintf("Failed to start Nextcloud auth: %v", err))
		return
	}

	log.Printf("Login flow initiated, URL: %s", loginURL)

	// Send context and auth URL to client
	landmark := fmt.Sprintf("nextcloud://%s/auth", state.Identity)
	sendSetContext(state.Conn, 0, landmark)

	// Send the login URL as a clickable link
	vertexID := hash64("auth", state.Identity)
	sendSetVertexLabel(state.Conn, 0, vertexID, "text/x-url", []byte(loginURL))

	// Send a label explaining what to do
	instructionID := hash64("instruction", state.Identity)
	instruction := fmt.Sprintf("Click the link above to authorize this server to access your Nextcloud files.\n\nIdentity: %s", state.Identity)
	sendSetVertexLabel(state.Conn, 0, instructionID, "text/plain", []byte(instruction))

	// Send edges: instruction north, auth link south
	sendSetEdges(state.Conn, 0, instructionID, 0, 0, 0, vertexID, 0, 0, 0)
	sendSetEdges(state.Conn, 0, vertexID, 0, 0, instructionID, 0, 0, 0, 0)

	// Store poll state
	state.State = stateAwaitingAuth
	state.LoginPoll = &LoginPollState{
		PollEndpoint: pollEndpoint,
		PollToken:    pollToken,
		StopChan:     make(chan struct{}),
	}

	// Start polling in background
	go pollForAuthCompletion(state)
}

// pollForAuthCompletion polls until auth is complete or connection closes
func pollForAuthCompletion(state *ConnectionState) {
	ticker := time.NewTicker(2 * time.Second)
	defer ticker.Stop()

	for {
		select {
		case <-state.LoginPoll.StopChan:
			return
		case <-ticker.C:
			username, appPassword, done, err := pollLoginCompletion(
				state.LoginPoll.PollEndpoint,
				state.LoginPoll.PollToken,
			)
			if err != nil {
				log.Printf("Poll error: %v", err)
				continue
			}
			if done {
				log.Printf("Auth complete for %s (username: %s)", state.Identity, username)
				state.Username = username
				state.AppPassword = appPassword
				state.State = stateBrowsing

				// Store credentials
				credStoreMu.Lock()
				credStore.Set(state.Identity, &Credential{
					NextcloudURL: state.NextcloudURL,
					Username:     username,
					AppPassword:  appPassword,
				})
				credStore.Save()
				credStoreMu.Unlock()

				// Send root directory listing
				log.Printf("Sending directory listing after successful auth")
				sendDirectoryListing(state, "/")
				return
			}
		}
	}
}


// WebDAV types for PROPFIND response
type MultiStatus struct {
	XMLName   xml.Name   `xml:"multistatus"`
	Responses []Response `xml:"response"`
}

type Response struct {
	Href     string   `xml:"href"`
	PropStat PropStat `xml:"propstat"`
}

type PropStat struct {
	Prop   Prop   `xml:"prop"`
	Status string `xml:"status"`
}

type Prop struct {
	DisplayName  string `xml:"displayname"`
	ContentType  string `xml:"getcontenttype"`
	ContentLen   int64  `xml:"getcontentlength"`
	ResourceType struct {
		Collection *struct{} `xml:"collection"`
	} `xml:"resourcetype"`
}

// DavEntry represents a file or directory from WebDAV
type DavEntry struct {
	Name        string
	Path        string
	IsDir       bool
	ContentType string
	Size        int64
}

// listWebDAV lists a directory via WebDAV PROPFIND
func listWebDAV(nextcloudURL, username, appPassword, dirPath string) ([]DavEntry, error) {
	// Build WebDAV URL
	webdavURL := fmt.Sprintf("%s/remote.php/dav/files/%s%s",
		strings.TrimSuffix(nextcloudURL, "/"),
		username,
		dirPath,
	)

	req, err := http.NewRequest("PROPFIND", webdavURL, strings.NewReader(`<?xml version="1.0" encoding="UTF-8"?>
<d:propfind xmlns:d="DAV:">
  <d:prop>
    <d:displayname/>
    <d:getcontenttype/>
    <d:getcontentlength/>
    <d:resourcetype/>
  </d:prop>
</d:propfind>`))
	if err != nil {
		return nil, err
	}

	req.SetBasicAuth(username, appPassword)
	req.Header.Set("Depth", "1")
	req.Header.Set("Content-Type", "application/xml")

	client := &http.Client{Timeout: 30 * time.Second}
	resp, err := client.Do(req)
	if err != nil {
		return nil, fmt.Errorf("PROPFIND failed: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != 207 { // Multi-Status
		body, _ := io.ReadAll(resp.Body)
		return nil, fmt.Errorf("PROPFIND returned %d: %s", resp.StatusCode, string(body))
	}

	var ms MultiStatus
	if err := xml.NewDecoder(resp.Body).Decode(&ms); err != nil {
		return nil, fmt.Errorf("failed to decode PROPFIND response: %w", err)
	}

	var entries []DavEntry
	basePath := fmt.Sprintf("/remote.php/dav/files/%s", username)

	for _, r := range ms.Responses {
		// Skip the directory itself (first entry)
		entryPath := strings.TrimPrefix(r.Href, basePath)
		if entryPath == dirPath || entryPath == dirPath+"/" {
			continue
		}

		// Clean up the path
		entryPath = strings.TrimSuffix(entryPath, "/")
		name := path.Base(entryPath)

		isDir := r.PropStat.Prop.ResourceType.Collection != nil

		entries = append(entries, DavEntry{
			Name:        name,
			Path:        entryPath,
			IsDir:       isDir,
			ContentType: r.PropStat.Prop.ContentType,
			Size:        r.PropStat.Prop.ContentLen,
		})
	}

	return entries, nil
}

// getWebDAV fetches file content via WebDAV GET
func getWebDAV(nextcloudURL, username, appPassword, filePath string) ([]byte, string, error) {
	webdavURL := fmt.Sprintf("%s/remote.php/dav/files/%s%s",
		strings.TrimSuffix(nextcloudURL, "/"),
		username,
		filePath,
	)

	req, err := http.NewRequest("GET", webdavURL, nil)
	if err != nil {
		return nil, "", err
	}

	req.SetBasicAuth(username, appPassword)

	client := &http.Client{Timeout: 30 * time.Second}
	resp, err := client.Do(req)
	if err != nil {
		return nil, "", fmt.Errorf("GET failed: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != 200 {
		return nil, "", fmt.Errorf("GET returned %d", resp.StatusCode)
	}

	contentType := resp.Header.Get("Content-Type")
	if contentType == "" {
		contentType = "application/octet-stream"
	}

	// Limit read to 1MB
	content, err := io.ReadAll(io.LimitReader(resp.Body, 1024*1024))
	if err != nil {
		return nil, "", fmt.Errorf("failed to read content: %w", err)
	}

	return content, contentType, nil
}
