# Gradesta Browser Testing Manual

## Overview

This manual documents all screens, sidebars, key functionality, and test flows for the Gradesta Browser application. Use this guide for systematic manual testing.

---

## 1. MAIN LAYOUT STRUCTURE

The browser uses an egui-based layout with four main regions:

| Region | Location | Purpose |
|--------|----------|---------|
| **Top Panel** | Top of window | Server/Landmark URL input, connection controls |
| **Bottom Panel** | Bottom of window | Help text, quick action buttons |
| **Right Side Panel** | Right side (400px min) | Dynamic content display, modals, settings |
| **Central Panel** | Remaining space | Grid view for vertex navigation |

---

## 2. SCREENS AND PANELS

### 2.1 Top Panel (URL Bar)

**Components:**
- Server input field (hint: `ws://localhost:8080`)
- Landmark input field (hint: `/`)
- Connect/Refresh button (toggles based on connection state)
- Copy URL button (📋)
- Status message display
- Current landmark display
- Zoom level indicator

**Test Flows:**
- [ ] Enter server URL and connect
- [ ] Enter landmark path and navigate
- [ ] Press Enter in either field to trigger connection
- [ ] Click Refresh when already connected
- [ ] Click 📋 to copy full URL
- [ ] Verify zoom % updates when zooming
- [ ] Paste full URL (smart paste should split into server/landmark)
- [ ] Ctrl+Shift+C to copy full URL when focused

### 2.2 Bottom Panel (Help & Actions)

**Components:**
- Navigation help text: `↑↓←→ Nav | Enter=Click | Space=Record | I=Edit | Y=Yank | Ctrl+K=Keybindings`
- TTS toggle button (🔊/🔇)
- Debug panel toggle (🐛)
- Keybindings button (⌨)
- Identities button (🔑) with count
- Bag button (📋) with item count

**Test Flows:**
- [ ] Click TTS button to toggle text-to-speech
- [ ] Click Debug button to show/hide debug panel
- [ ] Click Keybindings to open keybindings editor
- [ ] Click Identities to manage identities
- [ ] Click Bag to open vertex clipboard
- [ ] Verify counts update correctly

### 2.3 Central Panel (Grid View)

**States:**
1. **Disconnected**: "Enter a WebSocket URL above and click Connect"
2. **Awaiting data**: "Waiting for data from server..."
3. **Pending auth**: "Server requires authentication"
4. **Connected**: Shows vertex grid with current cell highlighted

**Components:**
- Vertex cards with thumbnails
- Edge lines (North-South: cyan, East-West: magenta)
- Direction arrow on current cell
- Ghost edge indicators (purple)

**Test Flows:**
- [ ] Navigate with arrow keys (↑↓←→)
- [ ] Navigate with WASD keys
- [ ] Navigate up/down stack (PageUp/PageDown)
- [ ] Click on adjacent vertex to navigate
- [ ] Verify current cell is highlighted
- [ ] Verify edge lines connect vertices
- [ ] Test zoom in/out (Ctrl++, Ctrl+-, Ctrl+0)
- [ ] Test pinch-to-zoom gesture
- [ ] Test Ctrl+scroll wheel zoom

---

## 3. SIDEBAR MODES

### 3.1 Preview Mode (Default)
**Header:** "Content Preview"

Displays current vertex content based on MIME type (text, image, audio, video).

**Test Flows:**
- [ ] View text content
- [ ] View image content
- [ ] View audio content with waveform
- [ ] View video content
- [ ] Verify portal links are clickable

### 3.2 Text View Mode
**Header:** "Text Content"
**Trigger:** Click text content or navigate to text vertex

**Components:**
- Close button (✕)
- Scrollable text display (monospace)
- Hint: "Ctrl+Enter: Fullscreen | Escape: Close"

**Test Flows:**
- [ ] View long text with scrolling
- [ ] Press Escape to close
- [ ] Press Ctrl+Enter for fullscreen
- [ ] Close from fullscreen

### 3.3 Image View Mode
**Header:** "Image"
**Trigger:** Click image content or navigate to image vertex

**Components:**
- Close button (✕)
- Scrollable image display
- Animated GIF playback

**Test Flows:**
- [ ] View static image (PNG, JPEG, WebP)
- [ ] View animated GIF
- [ ] Scroll large images
- [ ] Press Ctrl+Enter for fullscreen
- [ ] Press Escape to close

### 3.4 Video Play Mode
**Header:** (Video display)
**Trigger:** Click video content or navigate to video vertex

**Components:**
- Video frame display
- Seek bar with progress
- Play/Pause button
- Time display (current / duration)

**Test Flows:**
- [ ] Play video
- [ ] Pause video
- [ ] Seek to position
- [ ] Press Ctrl+Enter for fullscreen
- [ ] Press Escape to close (stops playback)

### 3.5 Text Input Mode
**Header:** "New Text Note" or "Edit Text"
**Trigger:** Press `N` (new) or `I` (edit)

**Components:**
- Direction indicator (if creating new)
- Large monospace textarea
- Save button (Ctrl+Enter)
- Cancel button (✕)
- Hint: "Ctrl+Enter: Save | Escape: Cancel"

**Test Flows:**
- [ ] Create new text vertex (N key)
- [ ] Edit existing text (I key)
- [ ] Type and save with Ctrl+Enter
- [ ] Cancel with Escape
- [ ] Verify direction indicator shows correctly
- [ ] Test copy/paste (Ctrl+C/V)
- [ ] Test select all (Ctrl+A)
- [ ] Test undo/redo (Ctrl+Z/Ctrl+Shift+Z)

### 3.6 Recording Mode (Blocking)
**Header:** "Recording Audio"
**Trigger:** Hold Space key

**Components:**
- Red circle indicator (🔴)
- Elapsed time display
- Audio level meter
- Instructions: "Release Space to save"
- Cancel button (Escape)

**Test Flows:**
- [ ] Hold Space to start recording
- [ ] Verify level meter responds to audio
- [ ] Release Space to save
- [ ] Press Escape to cancel
- [ ] Verify transcription appears in layer 1

### 3.7 Identity Panel
**Header:** "Identity Management"
**Trigger:** Click 🔑 button or Ctrl+I

**Components:**
- Close button (✕)
- List of identities with Remove buttons
- Nextcloud URL input
- "Connect Nextcloud Account" button

**Test Flows:**
- [ ] View existing identities
- [ ] Remove an identity
- [ ] Enter Nextcloud URL
- [ ] Initiate Nextcloud login
- [ ] Complete login flow in browser
- [ ] Set display name for new identity
- [ ] Create identity

### 3.8 Identification Request (Blocking)
**Header:** "Identification Request"
**Trigger:** Server requests authentication

**Components:**
- Server URL display
- Reason for identification
- Identity selector dropdown
- "Remember this server" checkbox
- Accept button (Enter)
- Refuse button (Escape)

**Test Flows:**
- [ ] Cycle through identities (Tab)
- [ ] Accept identification (Enter)
- [ ] Accept and remember (Ctrl+Enter)
- [ ] Refuse identification (Escape)

### 3.9 Bag Panel (Vertex Clipboard)
**Header:** "Bag"
**Trigger:** Click 📋 button or Ctrl+B

**Components:**
- Close button (✕)
- Clear All button
- Keyboard hints: "Y=Yank | P=Paste | G=Go | Ctrl+Y=Pop"
- Items list (newest first with "→ Next to paste:" marker)
- Remove button per item

**Test Flows:**
- [ ] Yank vertex to bag (Y key)
- [ ] View bag contents
- [ ] Click item to jump to vertex
- [ ] Remove individual item
- [ ] Clear all items
- [ ] Pop from bag (Ctrl+Y)
- [ ] Paste from bag (P key)
- [ ] Go to bag top (G key)

### 3.10 Navigation Panel
**Header:** "Navigation"
**Trigger:** Ctrl+N

**Components:**
- Close button (✕)
- Landmark History section
- Current landmark marker (→)
- Vertex cards for each landmark

**Test Flows:**
- [ ] View landmark history
- [ ] Click landmark to jump
- [ ] Verify current landmark is marked

### 3.11 Keybindings Editor
**Header:** "Keybindings"
**Trigger:** Click ⌨ button or Ctrl+K

**Components:**
- Close button (✕)
- Filter/category buttons
- List of keybindings
- Preset selector (Normal, Vim, Emacs)
- Save button

**Test Flows:**
- [ ] View all keybindings
- [ ] Filter by category
- [ ] Switch preset
- [ ] Modify a keybinding
- [ ] Save changes

### 3.12 Debug Panel
**Header:** "Debug Log"
**Trigger:** Click 🐛 button

**Components:**
- Close button (✕)
- Log file path
- Clear button with entry count
- Filter tabs: All, Cmd, Ctx, Exec, Key
- Scrollable log entries

**Test Flows:**
- [ ] View debug log
- [ ] Filter by category
- [ ] Clear log
- [ ] Verify new entries appear

---

## 4. COMMAND BAR (Vim-style)

**Trigger:** Press `:` (colon)
**Location:** Bottom center overlay

**Components:**
- Command input with ":" prefix
- Fuzzy search results (up to 5)
- Keybinding display per command

**Test Flows:**
- [ ] Open with `:` key
- [ ] Type to search commands
- [ ] Navigate with arrow keys
- [ ] Tab to autocomplete
- [ ] Enter to execute
- [ ] Escape to close

---

## 5. INPUT MODES

### 5.1 Normal Mode (Default)
All navigation and graph commands available.

**Key Commands:**
| Key | Action |
|-----|--------|
| ↑/W | Navigate north |
| ↓/S | Navigate south |
| ←/A | Navigate west |
| →/D | Navigate east |
| PageUp | Navigate up (stack) |
| PageDown | Navigate down (stack) |
| Enter | Click/activate vertex |
| Space (hold) | Start recording |
| I | Edit current vertex |
| N | New text vertex |
| Y | Yank to bag |
| P | Paste from bag |
| C | Cut edge |
| G | Go to bag top |
| Backspace | History back |
| Delete | Delete vertex |

### 5.2 Text Input Mode
**Trigger:** N or I key

**Key Commands:**
| Key | Action |
|-----|--------|
| Ctrl+Enter | Save |
| Escape | Cancel |
| Ctrl+C | Copy |
| Ctrl+X | Cut |
| Ctrl+V | Paste |
| Ctrl+A | Select all |
| Ctrl+Z | Undo |
| Ctrl+Shift+Z | Redo |

### 5.3 Recording Mode
**Trigger:** Hold Space

**Key Commands:**
| Key | Action |
|-----|--------|
| Space (release) | Save recording |
| Escape | Cancel |

---

## 6. GLOBAL SHORTCUTS

| Key | Action |
|-----|--------|
| Ctrl+L | Focus URL bar |
| Ctrl+B | Toggle bag panel |
| Ctrl+N | Toggle navigation panel |
| Ctrl+K | Open keybindings |
| Ctrl+T | Toggle TTS |
| Ctrl+Enter | Toggle fullscreen |
| Ctrl++ | Zoom in |
| Ctrl+- | Zoom out |
| Ctrl+0 | Reset zoom |
| Ctrl+] | TTS speed up |
| Ctrl+[ | TTS speed down |
| F5 | Refresh |
| : | Open command bar |
| Escape | Close modal/fullscreen |

---

## 7. FEATURE TEST FLOWS

### 7.1 Connection Flow
1. [ ] Enter server URL (e.g., `ws://localhost:8080`)
2. [ ] Enter landmark (e.g., `/`)
3. [ ] Click Connect or press Enter
4. [ ] Verify "Connecting..." status
5. [ ] Verify grid populates with vertices
6. [ ] Verify current landmark displays

### 7.2 Navigation Flow
1. [ ] Navigate to adjacent vertex with arrow keys
2. [ ] Verify current cell highlight moves
3. [ ] Navigate through a portal
4. [ ] Use Backspace to go back
5. [ ] Navigate up/down in stack

### 7.3 Content Creation Flow
1. [ ] Set direction (Shift+Arrow)
2. [ ] Press N for new text
3. [ ] Type content
4. [ ] Press Ctrl+Enter to save
5. [ ] Verify new vertex created in correct direction

### 7.4 Audio Recording Flow
1. [ ] Set direction (Shift+Arrow)
2. [ ] Hold Space to record
3. [ ] Speak
4. [ ] Release Space
5. [ ] Verify audio vertex created
6. [ ] Wait for transcription in layer 1

### 7.5 Bag (Clipboard) Flow
1. [ ] Navigate to vertex
2. [ ] Press Y to yank
3. [ ] Navigate elsewhere
4. [ ] Press P to paste
5. [ ] Verify connection created
6. [ ] Press G to go to bag top
7. [ ] Press Ctrl+Y to pop from bag

### 7.6 Identity & Authentication Flow
1. [ ] Click 🔑 to open identity panel
2. [ ] Enter Nextcloud URL
3. [ ] Click Connect
4. [ ] Complete login in browser
5. [ ] Enter display name
6. [ ] Create identity
7. [ ] Connect to server requiring auth
8. [ ] Select identity when prompted
9. [ ] Accept identification

### 7.7 TTS Flow
1. [ ] Press Ctrl+T to enable TTS
2. [ ] Navigate to text vertex
3. [ ] Verify text is spoken
4. [ ] Press Ctrl+] to speed up
5. [ ] Press Ctrl+[ to slow down
6. [ ] Press Ctrl+T to disable

### 7.8 Fullscreen Flow
1. [ ] Navigate to content vertex
2. [ ] Press Ctrl+Enter for fullscreen
3. [ ] Verify content fills screen
4. [ ] Press Escape to exit
5. [ ] Verify returns to normal view

---

## 8. EDGE CASES TO TEST

### Connection
- [ ] Invalid server URL
- [ ] Server not responding
- [ ] Connection dropped mid-session
- [ ] Rapid connect/disconnect

### Navigation
- [ ] Navigate to non-existent direction
- [ ] Circular graph navigation
- [ ] Very large grid (many vertices)
- [ ] Deep stack traversal

### Content
- [ ] Very long text content
- [ ] Large image files
- [ ] Animated GIF with many frames
- [ ] Corrupted media files
- [ ] Missing media data

### Recording
- [ ] Very short recording (<1 second)
- [ ] Long recording
- [ ] Cancel recording immediately
- [ ] No microphone available

### Clipboard
- [ ] Yank same vertex twice
- [ ] Paste with empty bag
- [ ] Clear bag while viewing
- [ ] Bag with many items

### Identity
- [ ] Cancel Nextcloud login
- [ ] Invalid Nextcloud URL
- [ ] Refuse identification
- [ ] Multiple identities for same server

---

## 9. KEYBINDING PRESETS

### Normal (Default)
Standard Windows/Linux keybindings with arrow keys.

### Vim-like
- HJKL for navigation
- U for history back
- O for new vertex
- X for delete

### Emacs-like
- Ctrl+F/B/N/P for navigation
- Alt+V/Ctrl+V for page nav
- Ctrl+O for new vertex
- Alt+W for yank, Ctrl+Y for paste

**Test Flow:**
1. [ ] Open keybindings (Ctrl+K)
2. [ ] Select Vim preset
3. [ ] Verify HJKL navigation works
4. [ ] Select Emacs preset
5. [ ] Verify Ctrl+F/B/N/P works
6. [ ] Return to Normal preset

---

## 10. FILE LOCATIONS FOR REFERENCE

| Component | File |
|-----------|------|
| Top/Bottom Panels | `src/ui/panels.rs` |
| Grid View | `src/ui/grid.rs` |
| Sidebar Content | `src/ui/sidebar_content.rs` |
| Fullscreen | `src/ui/fullscreen.rs` |
| Command Bar | `src/ui/command_bar.rs` |
| Input Handling | `src/ui/input.rs` |
| Commands | `src/commands/mod.rs` |
| Network | `src/network.rs` |
| Media | `src/media.rs` |
| Identity | `src/identity.rs` |
| TTS | `src/tts.rs` |
| Whisper | `src/whisper.rs` |
