version = 1
provider = "recall_ai"
categories = ["meetings", "recording", "transcription"]

command "create_bot" {
  title       = "Create bot"
  summary     = "Deploy a recording bot to join a video meeting"
  description = <<-EOT
    Creates a recall.ai bot that joins and records a video meeting.
    Supports Zoom, Google Meet, Microsoft Teams, Webex, and GoToMeeting.

    For reliable joins, schedule the bot at least 10 minutes ahead using join_at.
    Ad-hoc bots (no join_at) join immediately but may fail during peak usage.

    Parameters:
    - meeting_url: Full meeting invite URL (e.g. https://zoom.us/j/123456789)
    - bot_name: Display name shown to meeting participants (default: Meeting Notetaker)
    - join_at: ISO 8601 timestamp for scheduled join (e.g. 2026-02-23T15:00:00Z)
    - language_code: Transcription language (default: en_us)

    ## Guidance for AI agents
    Use this command to start recording a meeting. Save the returned id (bot_id) —
    you need it for all subsequent operations. After creating the bot, poll
    recall_ai.get_bot every 10-15 seconds until status reaches "joined", then call
    recall_ai.start_recording.
    Example: `earl call --yes --json recall_ai.create_bot --meeting_url https://zoom.us/j/123 --bot_name "Notetaker"`
  EOT

  annotations {
    mode    = "write"
    secrets = ["recall_ai.api_key"]
  }

  param "meeting_url" {
    type        = "string"
    required    = true
    description = "Full meeting invite URL (Zoom, Google Meet, Teams, Webex, GoToMeeting)"
  }

  param "bot_name" {
    type        = "string"
    required    = false
    default     = "Meeting Notetaker"
    description = "Display name shown to meeting participants (max 100 chars)"
  }

  param "join_at" {
    type        = "string"
    required    = false
    default     = ""
    description = "ISO 8601 timestamp for scheduled join — omit to join immediately. Use a time at least 10 minutes in the future for reliability."
  }

  param "language_code" {
    type        = "string"
    required    = false
    default     = "en_us"
    description = "Transcription language code (e.g. en_us, fr, es, de)"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://api.recall.ai/api/v1/bot/"

    auth {
      kind   = "bearer"
      secret = "recall_ai.api_key"
    }

    headers = {
      Accept = "application/json"
    }

    body {
      kind = "json"
      value = {
        meeting_url = "{{ args.meeting_url }}"
        bot_name    = "{{ args.bot_name }}"
        join_at     = "{{ args.join_at if args.join_at else none }}"
        recording_config = {
          transcript = {
            provider = {
              recallai_streaming = {
                language_code = "{{ args.language_code }}"
              }
            }
          }
        }
      }
    }
  }

  result {
    decode = "json"
    output = "Bot created: {{ result.id }}\nName: {{ result.bot_name }}\nMeeting: {{ result.meeting_url }}{% if result.join_at %}\nScheduled: {{ result.join_at }}{% endif %}\n\nSave this bot_id for all subsequent calls: {{ result.id }}"
  }
}

command "get_bot" {
  title       = "Get bot"
  summary     = "Get bot status and artifact IDs"
  description = <<-EOT
    Retrieves full bot details including lifecycle status and media_shortcuts,
    which contain the IDs needed to retrieve transcripts, video, and audio.

    Parameters:
    - bot_id: UUID of the bot (from create_bot response)

    Bot status progression:
      pending -> joining -> joined -> recording -> stopped -> done

    Artifact status values: waiting | processing | done | failed | deleted

    ## Guidance for AI agents
    Poll this command to monitor bot progress. When status is "done" and
    media_shortcuts.transcript.status.code is "done", the transcript is ready.
    Use the IDs in media_shortcuts to call get_transcript, get_video, get_audio.
    Example: `earl call --yes --json recall_ai.get_bot --bot_id <id>`
  EOT

  annotations {
    mode    = "read"
    secrets = ["recall_ai.api_key"]
  }

  param "bot_id" {
    type        = "string"
    required    = true
    description = "Bot UUID from create_bot"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.recall.ai/api/v1/bot/{{ args.bot_id }}/"

    auth {
      kind   = "bearer"
      secret = "recall_ai.api_key"
    }

    headers = {
      Accept = "application/json"
    }
  }

  result {
    decode = "json"
    output = "Bot {{ result.id }} [{{ result.status | default('unknown') }}]\nMeeting: {{ result.meeting_url }}\nName: {{ result.bot_name }}{% if result.join_at %}\nScheduled: {{ result.join_at }}{% endif %}\n\nArtifacts:\n  Transcript: {{ result.media_shortcuts.transcript.status.code | default('n/a') }} (id: {{ result.media_shortcuts.transcript.id | default('none') }})\n  Video:      {{ result.media_shortcuts.video_mixed.status.code | default('n/a') }} (id: {{ result.media_shortcuts.video_mixed.id | default('none') }})\n  Audio:      {{ result.media_shortcuts.audio_mixed.status.code | default('n/a') }} (id: {{ result.media_shortcuts.audio_mixed.id | default('none') }})"
  }
}

command "list_bots" {
  title       = "List bots"
  summary     = "List recall.ai bots with optional filters"
  description = <<-EOT
    Lists bots in the workspace. Use join_at_after to filter for upcoming scheduled
    bots, or leave blank to list all bots.

    Parameters:
    - join_at_after: ISO 8601 timestamp — only return bots scheduled after this time
    - page: Page number for pagination (default: 1)

    ## Guidance for AI agents
    Use this to find a bot_id when you don't have it. Filter by join_at_after to
    find future scheduled bots. Sort the response by join_at or created_at to find
    the most recent bot.
    Example: `earl call --yes --json recall_ai.list_bots`
  EOT

  annotations {
    mode    = "read"
    secrets = ["recall_ai.api_key"]
  }

  param "join_at_after" {
    type        = "string"
    required    = false
    default     = ""
    description = "ISO 8601 timestamp — only return bots scheduled after this time"
  }

  param "page" {
    type        = "integer"
    required    = false
    default     = 1
    description = "Page number for pagination"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.recall.ai/api/v1/bot/"

    auth {
      kind   = "bearer"
      secret = "recall_ai.api_key"
    }

    query = {
      join_at_after = "{{ args.join_at_after }}"
      page          = "{{ args.page }}"
    }

    headers = {
      Accept = "application/json"
    }
  }

  result {
    decode = "json"
    output = "{{ result | length }} bot(s):\n{% for bot in result %}  {{ bot.id }} [{{ bot.status | default('?') }}] {{ bot.bot_name }} — {{ bot.meeting_url }}{% if bot.join_at %} (scheduled: {{ bot.join_at }}){% endif %}\n{% endfor %}"
  }
}

command "delete_bot" {
  title       = "Delete bot"
  summary     = "Delete a scheduled bot before it joins"
  description = <<-EOT
    Deletes a scheduled bot. Only works if the bot has not yet joined the meeting.
    Use this to cancel a scheduled recording or clean up stale bots.

    WARNING: This permanently deletes the bot and any associated artifacts.
    Do not delete bots that are currently in a call — use leave_call first.

    Parameters:
    - bot_id: UUID of the bot to delete

    ## Guidance for AI agents
    Only call this for bots that are in "pending" status (not yet joined).
    For bots currently in a meeting, call leave_call first, then delete after
    status reaches "done".
    Example: `earl call --yes --json recall_ai.delete_bot --bot_id <id>`
  EOT

  annotations {
    mode    = "write"
    secrets = ["recall_ai.api_key"]
  }

  param "bot_id" {
    type        = "string"
    required    = true
    description = "Bot UUID to delete"
  }

  operation {
    protocol = "http"
    method   = "DELETE"
    url      = "https://api.recall.ai/api/v1/bot/{{ args.bot_id }}/"

    auth {
      kind   = "bearer"
      secret = "recall_ai.api_key"
    }
  }

  result {
    decode = "json"
    output = "Bot {{ args.bot_id }} deleted."
  }
}
