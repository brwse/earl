---
name: recall_ai
description: Use recall.ai to record video meetings, retrieve transcripts, and access recordings. Use when the user wants to record a meeting, get a transcript, summarize a call, or access meeting audio/video.
---

# Recall.ai

Records video meetings (Zoom, Google Meet, Teams, Webex, GoToMeeting), generates
speaker-attributed transcripts, and produces downloadable audio/video files.

## Status Reference

**Bot lifecycle status** (from `get_bot`):
| Status | Meaning |
|--------|---------|
| `pending` | Bot created, not yet attempting to join |
| `joining` | Bot is connecting to the meeting |
| `joined` | Bot is in the meeting, recording NOT started |
| `recording` | Bot is actively recording |
| `stopped` | Recording stopped, processing in progress |
| `done` | Bot left and all processing complete |

**Artifact status** (`media_shortcuts.*.status.code`):
| Status | Meaning |
|--------|---------|
| `waiting` | Not started |
| `processing` | Being generated |
| `done` | Ready to retrieve |
| `failed` | Generation failed — retry or contact support |
| `deleted` | Artifact was removed |

---

## CRITICAL: Async Lifecycle

Creating a bot does NOT start recording. The full sequence is async:

```
create_bot → [wait: joining] → start_recording → [meeting runs] → leave_call/stop_recording → [wait: processing] → get_transcript → download_transcript
```

**Do NOT call `get_transcript` immediately after `create_bot`.** You will get an error or empty data. Always poll first.

**Polling intervals:**
- While status is `joining`: every 10–15 seconds (max 2 minutes — if longer, the meeting URL may be invalid or the meeting hasn't started)
- While status is `recording`: every 30 seconds
- After `stop_recording` or `leave_call`, while transcript is `processing`: every 15 seconds

---

## Recipe A: Record a meeting and get the transcript

```bash
# 1. Create the bot (schedule 10+ minutes ahead for reliability)
earl call --yes --json recall_ai.create_bot \
  --meeting_url "https://zoom.us/j/123456789" \
  --bot_name "Notetaker" \
  --join_at "2026-02-23T15:00:00Z"
# → SAVE the returned id as bot_id

# 2. Poll until the bot has joined (repeat every 10–15s)
earl call --yes --json recall_ai.get_bot --bot_id <bot_id>
# → Wait until status == "joined"

# 3. Start recording
earl call --yes --json recall_ai.start_recording --bot_id <bot_id>

# 4. (Meeting runs — poll every 30s if you need to monitor)
earl call --yes --json recall_ai.get_bot --bot_id <bot_id>

# 5. When the meeting ends, leave the call (or let automatic_leave trigger)
earl call --yes --json recall_ai.leave_call --bot_id <bot_id>

# 6. Poll until transcript is ready (repeat every 15s)
earl call --yes --json recall_ai.get_bot --bot_id <bot_id>
# → Wait until media_shortcuts.transcript.status.code == "done"
# → Read media_shortcuts.transcript.id from the response

# 7. Get transcript metadata (includes download URL)
earl call --yes --json recall_ai.get_transcript --transcript_id <transcript_id>
# → Read data.download_url from the response

# 8. Download full transcript text
earl call --yes --json recall_ai.download_transcript --url "<data.download_url>"
# → Returns speaker-attributed transcript text — summarize, extract action items, etc.
```

---

## Recipe B: Retrieve a video or audio recording

```bash
# 1. Get bot to find media IDs
earl call --yes --json recall_ai.get_bot --bot_id <bot_id>
# → Read media_shortcuts.video_mixed.id and media_shortcuts.audio_mixed.id
# → Check that status.code == "done" before proceeding

# 2. Get video download URL
earl call --yes --json recall_ai.get_video --video_id <video_mixed_id>
# → Returns data.download_url — share this link with the user (expires in ~5 hours)

# 3. Get audio download URL
earl call --yes --json recall_ai.get_audio --audio_id <audio_mixed_id>
# → Returns data.download_url — share this link with the user (expires in ~5 hours)
```

**Important:** Do not attempt to download or process binary video/audio content.
Present the download URL to the user as a clickable link.

---

## Recipe C: Control a live meeting

```bash
# Pause recording during a sensitive segment
earl call --yes --json recall_ai.pause_recording --bot_id <bot_id>

# Resume when ready
earl call --yes --json recall_ai.resume_recording --bot_id <bot_id>

# Send a message to meeting participants
earl call --yes --json recall_ai.send_chat_message \
  --bot_id <bot_id> \
  --message "Action items so far: 1) Alice to send report 2) Bob to schedule follow-up"
```

---

## Finding a lost bot_id

If you don't have the bot_id, use `list_bots` to find it:

```bash
# List all bots
earl call --yes --json recall_ai.list_bots

# List only future scheduled bots
earl call --yes --json recall_ai.list_bots --join_at_after "2026-02-23T00:00:00Z"
```

Sort the results by `join_at` or `created_at` to find the most recent bot.
Never create a duplicate bot for a meeting — check the list first.

---

## Scheduling constraint

For reliable joins, `join_at` must be at least **10 minutes in the future**.
Ad-hoc bots (no `join_at`) join immediately from a warm pool but may fail with
507 errors during peak usage. For production workflows, always schedule ahead.

---

## Download URL expiry

`data.download_url` values are pre-signed AWS S3 URLs that expire in approximately
**5 hours**. Retrieve and process transcript/media in the same session. Do not save
the URL for later use.
