# Recall.ai Agent Automation Design Document

> **Note for contributors:** This is an initial design draft and contains inconsistencies
> with the final implementation. For the authoritative spec see `examples/recall_ai.hcl`:
> - `recording_config.transcript.provider` is a **nested discriminated object** in the
>   final implementation (not a plain string as shown in sections 3.2 and 6.1 below)
> - The field is `media_shortcuts` (not `artifacts`) in the actual bot response
> - Base URL is `https://api.recall.ai/api/v1/` (not `us-east-1.recall.ai`)

**Date:** 2026-02-23
**Purpose:** Enable autonomous agents to automate meeting recording, transcription, and data extraction via recall.ai REST API
**Audience:** AI agents, developers building agent workflows, platform architects

---

## Executive Summary

This document defines the architectural patterns and operational requirements for autonomous agents to automate meeting recording using recall.ai. The scope covers the full bot lifecycle: creation → joining → recording → transcript extraction → cleanup.

Key insight: Agents operate in a **fire-and-forget async model** where bot creation and status monitoring are decoupled. Meeting timestamps are unpredictable, so agents must use either webhook subscriptions or polling to react to status changes.

---

## 1. Bot Lifecycle & State Machine

### 1.1 Happy Path Sequence

```
Agent Action                    Bot State              API Call
─────────────────────────────────────────────────────────────────────
Create bot config               [pending creation]     POST /bot
                                                       → Returns bot {id, status}

Poll/webhook status             [joining]              GET /bot/{id}
(wait for join)                                        (poll until joined)

Start recording                 [joined, recording]    POST /bot/{id}/start_recording

Monitor via poll/webhook         [recording]            GET /bot/{id}
(human is in meeting)                                  (watch ArtifactStatus)

Stop recording                  [stopping]             POST /bot/{id}/stop_recording

Leave call                       [left]                 POST /bot/{id}/leave_call

Poll transcript                 [processing→done]      GET /transcript/{id}
                                                       (poll artifact status)

Cleanup (optional)              [deleted]              DELETE /bot/{id}
                                                       (only for scheduled bots)
```

### 1.2 State Values

The bot object includes:
- **bot.status**: Main bot lifecycle state (`joining`, `joined`, `recording`, `stopped`, `done`)
- **ArtifactStatus**: Output states for transcripts, audio, video (`processing`, `done`, `failed`, `deleted`)

Agents should monitor both fields:
- `bot.status` for join/leave events
- Nested artifact statuses for transcript readiness

---

## 2. REST API Operations

### 2.1 Bot Management (Create, Read, Update, Delete)

| Operation | Method | Endpoint | Purpose | Side Effects |
|-----------|--------|----------|---------|--------------|
| List Bots | GET | `/bot` | Enumerate all bots | None (read-only) |
| Create Bot | POST | `/bot` | Initialize bot for meeting | Creates new bot resource |
| Get Bot | GET | `/bot/{id}` | Fetch current bot state | None (read-only) |
| Update Bot | PATCH | `/bot/{id}` | Modify scheduled bot config | Mutates scheduled bot |
| Delete Bot | DELETE | `/bot/{id}` | Remove scheduled bot | Deletes bot (scheduled only) |

### 2.2 Recording Control

| Operation | Method | Endpoint | Purpose |
|-----------|--------|----------|---------|
| Start Recording | POST | `/bot/{id}/start_recording` | Begin capturing audio/video |
| Stop Recording | POST | `/bot/{id}/stop_recording` | End capture, start processing |
| Pause Recording | POST | `/bot/{id}/pause_recording` | Suspend capture (resume later) |
| Resume Recording | POST | `/bot/{id}/resume_recording` | Resume paused capture |

### 2.3 Call Control

| Operation | Method | Endpoint | Purpose |
|-----------|--------|----------|---------|
| Leave Call | POST | `/bot/{id}/leave_call` | Bot exits meeting |
| Pin Participant | POST | `/bot/{id}/pin_participant` | Focus on specific speaker |
| Send Chat Message | POST | `/bot/{id}/send_chat_message` | Post message to meeting chat |

### 2.4 Media Output Management

| Operation | Method | Endpoint | Purpose |
|-----------|--------|----------|---------|
| Enable Audio Output | POST | `/bot/{id}/output_audio` | Configure audio capture |
| Disable Audio Output | DELETE | `/bot/{id}/output_audio` | Remove audio output |
| Enable Video Output | POST | `/bot/{id}/output_video` | Configure video capture |
| Disable Video Output | DELETE | `/bot/{id}/output_video` | Remove video output |
| Enable Screenshare | POST | `/bot/{id}/output_screenshare` | Capture screen content |
| Delete Bot Media | POST | `/bot/{id}/delete_media` | Remove recorded files |

### 2.5 Transcript Access

| Operation | Method | Endpoint | Purpose |
|-----------|--------|----------|---------|
| List Transcripts | GET | `/transcript` | Enumerate all transcripts |
| Get Transcript | GET | `/transcript/{id}` | Fetch specific transcript |

---

## 3. Bot Creation Configuration Schema

### 3.1 Required Parameters

```json
{
  "meeting_url": "string (required)",
  "bot_name": "string (optional, max 100 chars, default: 'Meeting Notetaker')",
  "join_at": "ISO 8601 timestamp (optional, min: 10 minutes in future for reliability)"
}
```

### 3.2 Recording Configuration Object

```json
{
  "recording_config": {
    "transcript": {
      "provider": "string (e.g., 'deepgram', 'assembly')",
      "language": "string (e.g., 'en')"
    },
    "video_output": {
      "format": "string (e.g., 'mp4')",
      "resolution": "object (width, height)"
    },
    "audio_output": {
      "format": "string (e.g., 'wav', 'mp3')",
      "bitrate": "integer (kbps)"
    },
    "automatic_leave": {
      "waiting_room_timeout": "integer (seconds)",
      "silence_timeout": "integer (seconds)",
      "bot_detection_timeout": "integer (seconds)"
    },
    "automatic_audio_output": {
      "url": "string (audio file to play)"
    },
    "automatic_video_output": {
      "url": "string (video file to display)"
    }
  }
}
```

### 3.3 Meeting Platform Support

recall.ai supports:
- Zoom
- Google Meet
- Microsoft Teams
- Webex
- GoToMeeting

### 3.4 Scheduled vs Ad-hoc Bots

- **Scheduled Bot:** Set `join_at` to future timestamp (min 10 minutes ahead for reliability)
- **Ad-hoc Bot:** Omit `join_at`; bot joins immediately (within seconds)

---

## 4. Status Monitoring Patterns

### 4.1 Polling Pattern (Simple)

Agent polls `GET /bot/{id}` every N seconds:

```
while bot.status != "done":
  GET /bot/{id}
  if bot.status == "joined":
    POST /bot/{id}/start_recording
  if bot.artifacts.transcript.status == "done":
    GET /transcript/{id}
    break
  sleep(5 seconds)
```

**Pros:** Simple, no infrastructure
**Cons:** Latency (5-30s delay), higher API traffic

### 4.2 Webhook Pattern (Recommended)

Agent registers webhook endpoint; recall.ai POSTs status changes:

```
POST /webhook/setup
  -> register callback URL

recall.ai → Agent (webhook POST)
  {
    "event": "bot.status_changed",
    "bot_id": "uuid",
    "status": "joined",
    "timestamp": "ISO 8601"
  }
```

**Pros:** Real-time, minimal API calls
**Cons:** Requires webhook infrastructure, network ingress

Webhook events expected:
- `bot.created`
- `bot.joined`
- `bot.recording_started`
- `bot.recording_stopped`
- `bot.left`
- `transcript.processing`
- `transcript.done`
- `artifact.failed`

(See `/reference/webhooks-overview` in API docs for authoritative list)

---

## 5. Operational Requirements for Agents

### 5.1 Minimal Agent Flow

```python
# 1. Create bot
bot = POST /bot {
  "meeting_url": "https://zoom.us/j/...",
  "bot_name": "Earl Data Extractor",
  "recording_config": { ... }
}
bot_id = bot.id

# 2. Wait for join (polling example)
while GET /bot/{bot_id}.status != "joined":
  sleep(2)

# 3. Start recording
POST /bot/{bot_id}/start_recording

# 4. Wait for meeting to end (via webhook or polling)
# Agent logic: human leaves meeting → bot detects via automatic_leave timeout

# 5. Get transcript
transcript = GET /transcript/{transcript_id}
process_transcript(transcript)

# 6. Cleanup (optional)
DELETE /bot/{bot_id}  # Only for scheduled bots
```

### 5.2 Error Handling

Agents should handle:

| Scenario | HTTP Status | Recovery |
|----------|-------------|----------|
| Invalid meeting URL | 400 | Validate URL before creation |
| Bot already in call | 409 | Retry or use different bot_id |
| Transcript still processing | 202 | Retry GET /transcript in 30s |
| Network timeout | 5xx | Exponential backoff, max 3 retries |
| Unauthorized (bad API key) | 401 | Validate auth token |
| Rate limited | 429 | Backoff per Retry-After header |

### 5.3 Idempotency & Retries

- **POST /bot creation:** Not idempotent by API; agent should track bot_id to avoid duplicates
- **GET operations:** Idempotent; safe to retry
- **Recording control (start/stop):** May be idempotent; check API docs for guarantees
- **Recommended:** Exponential backoff (1s, 2s, 4s, 8s) for transient 5xx errors

### 5.4 Resource Cleanup

Agents should:
- Delete scheduled bots via `DELETE /bot/{id}` when no longer needed
- Not call `DELETE /bot/{id}` on bots still in active calls
- Wait for transcript artifact status == "done" before deleting bot

---

## 6. Integration with Earl (Hypothetical)

If integrating recall.ai into the Earl agent framework:

### 6.1 Operation Template

```hcl
operation "record_meeting" {
  protocol = "http"
  method = "POST"
  url = "https://us-east-1.recall.ai/api/v1/bot"

  body {
    kind = "json"
    content = jsonencode({
      meeting_url = var.meeting_url
      bot_name = "Earl Recorder"
      recording_config = {
        transcript = {
          provider = "deepgram"
          language = "en"
        }
      }
    })
  }

  auth {
    kind = "header"
    name = "Authorization"
    value = "Bearer ${var.recall_api_key}"
  }
}
```

### 6.2 Polling Skill

```hcl
# Wait for transcript ready
operation "get_transcript" {
  protocol = "http"
  method = "GET"
  url = "https://us-east-1.recall.ai/api/v1/transcript/${var.transcript_id}"

  auth {
    kind = "header"
    name = "Authorization"
    value = "Bearer ${var.recall_api_key}"
  }
}
```

---

## 7. Security Considerations

### 7.1 API Authentication

- **Method:** Bearer token in `Authorization: Bearer <api_key>` header
- **Key Management:** Store API key in secret manager (Earl's `secrets set` or external)
- **Rotation:** Implement key rotation policy (e.g., quarterly)

### 7.2 Meeting URL Safety

- Validate `meeting_url` is from known/trusted domains
- Avoid recording private/sensitive meetings without consent
- Implement access control (who can trigger recordings)

### 7.3 Data Residency

- Check recall.ai's regional endpoints (e.g., `us-east-1.recall.ai`)
- Ensure GDPR/data sovereignty compliance for transcript storage
- Consider transcript encryption in transit

### 7.4 Rate Limiting

- recall.ai may enforce per-API-key rate limits
- Implement client-side backoff to avoid throttling
- Monitor 429 responses and adjust polling frequency

---

## 8. Monitoring & Observability

### 8.1 Metrics to Track

- **Bot creation latency** (POST /bot → bot.id returned)
- **Join latency** (bot created → bot.status == "joined")
- **Recording duration** (start_recording → stop_recording)
- **Transcript processing time** (bot left → transcript.status == "done")
- **API error rate** (4xx, 5xx responses)
- **Webhook delivery latency** (event fired → agent received)

### 8.2 Logging

- Log bot_id with every API call (for debugging)
- Log state transitions (e.g., "bot {id} joined at {timestamp}")
- Log errors with full HTTP response (status, headers, body)
- Avoid logging API keys or meeting URLs in plaintext

### 8.3 Alerting

- Alert if transcript processing exceeds 5 minutes
- Alert on repeated 5xx errors
- Alert on webhook delivery failures (if using webhooks)
- Alert on bots stuck in "joining" state > 2 minutes

---

## 9. Known Limitations & Caveats

1. **Meeting URL Prediction:** Agents cannot predict meeting start times; use polling/webhooks to detect when bot joins
2. **No Batch Operations:** API requires per-bot operations; no bulk recording
3. **Transcript Latency:** Processing can take 1-5 minutes depending on meeting length
4. **Bot Detection Timeout:** Bot may be kicked from some meetings with anti-bot policies
5. **Webhook Reliability:** In critical paths, combine webhooks with periodic polling
6. **Scheduled Bot Limitation:** `DELETE /bot/{id}` only works on scheduled bots; ad-hoc bots auto-delete

---

## 10. Future Enhancements

- [ ] Real-time speaker identification in transcript
- [ ] Custom output formats (SRT, VTT for captions)
- [ ] Meeting recording snapshots (per-speaker engagement metrics)
- [ ] Transcript search/tagging API
- [ ] Webhook signature verification (HMAC-SHA256)
- [ ] Batch transcript export
- [ ] Custom recording quality profiles

---

## Appendix A: API Response Examples

### A.1 Create Bot Response

```json
{
  "id": "e13e70e8-79fb-4370-99ba-4e3143f7c0fb",
  "created_at": "2026-02-23T14:30:00Z",
  "meeting_url": "https://zoom.us/j/123456789",
  "bot_name": "Earl Recorder",
  "join_at": null,
  "status": "pending",
  "recording_config": {
    "transcript": {
      "provider": "deepgram",
      "language": "en"
    }
  },
  "artifacts": {
    "transcript": {
      "id": "t_12345",
      "status": "waiting",
      "created_at": null
    },
    "audio": { "status": "waiting" },
    "video": { "status": "waiting" }
  }
}
```

### A.2 Get Bot Response (Joined & Recording)

```json
{
  "id": "e13e70e8-79fb-4370-99ba-4e3143f7c0fb",
  "status": "recording",
  "joined_at": "2026-02-23T14:35:10Z",
  "artifacts": {
    "transcript": {
      "id": "t_12345",
      "status": "processing",
      "created_at": "2026-02-23T14:35:10Z"
    }
  }
}
```

### A.3 Get Transcript Response

```json
{
  "id": "t_12345",
  "status": "done",
  "created_at": "2026-02-23T14:40:05Z",
  "content": "Speaker 1: Good morning everyone... [full transcript text]",
  "word_count": 1240,
  "duration_seconds": 1800
}
```

---

## Appendix B: Glossary

- **Bot:** recall.ai's agent that joins a meeting and records
- **Artifact:** Output of recording (transcript, audio, video)
- **Scheduled Bot:** Bot created with `join_at` timestamp; joins automatically
- **Ad-hoc Bot:** Bot created without `join_at`; joins on creation
- **Transcript:** Text record of meeting dialogue (generated via speech-to-text)
- **Status Polling:** Agent periodically queries `GET /bot/{id}` for state
- **Webhook:** recall.ai pushes status events to agent's callback URL
- **TOCTOU:** Time-of-check-time-of-use; race condition (not applicable here)

---

## Appendix C: Related Documentation

- [recall.ai API Reference](https://docs.recall.ai/reference)
- [recall.ai Webhooks](https://docs.recall.ai/reference/webhooks-overview)
- [recall.ai Bot Lifecycle Guide](https://docs.recall.ai/guides) (if available)

---

**Document Version:** 1.0
**Last Updated:** 2026-02-23
**Status:** Draft (Ready for Review)
