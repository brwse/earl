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
    description = "ISO 8601 timestamp for scheduled join (e.g. 2026-02-23T15:00:00Z). Use a time at least 10 minutes in the future for reliability. NOTE: Earl always includes this field in the request body; if the recall.ai API rejects an empty string for this field, always provide a valid timestamp."
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
        join_at     = "{{ args.join_at or '' }}"
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
    output = <<-EOT
      Bot {{ result.id }} [{{ result.status | default('unknown') }}]
      Meeting: {{ result.meeting_url }}
      Name: {{ result.bot_name }}{% if result.join_at %}
      Scheduled: {{ result.join_at }}{% endif %}

      Artifacts:
      {% if result.media_shortcuts %}  Transcript: {{ result.media_shortcuts.transcript.status.code | default('n/a') }} (id: {{ result.media_shortcuts.transcript.id | default('none') }})
        Video:      {{ result.media_shortcuts.video_mixed.status.code | default('n/a') }} (id: {{ result.media_shortcuts.video_mixed.id | default('none') }})
        Audio:      {{ result.media_shortcuts.audio_mixed.status.code | default('n/a') }} (id: {{ result.media_shortcuts.audio_mixed.id | default('none') }}){% else %}  (not yet available — bot is still joining){% endif %}
    EOT
  }
}

command "list_bots" {
  title       = "List bots"
  summary     = "List recall.ai bots with optional filters"
  description = <<-EOT
    Lists bots in the workspace. Use join_at_after to filter for upcoming scheduled
    bots, or omit to list all bots.

    Parameters:
    - join_at_after: ISO 8601 timestamp — only return bots scheduled after this time
    - page: Page number for pagination (default: 1)

    NOTE: Earl always sends join_at_after in the query string. If omitted, it sends
    ?join_at_after= (empty string). If the recall.ai API rejects an empty datetime
    filter, always provide a valid ISO 8601 value for this parameter.

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
    output = "{{ result.results | length }} bot(s) (total: {{ result.count }}):\n{% for bot in result.results %}  {{ bot.id }} [{{ bot.status | default('?') }}] {{ bot.bot_name }} — {{ bot.meeting_url }}{% if bot.join_at %} (scheduled: {{ bot.join_at }}){% endif %}\n{% endfor %}"
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
    output = "Bot {{ args.bot_id }} deleted."
  }
}

command "start_recording" {
  title       = "Start recording"
  summary     = "Begin recording in an active bot session"
  description = <<-EOT
    Starts audio/video recording for a bot that has joined a meeting.
    Call this after get_bot shows status == "joined".

    Parameters:
    - bot_id: UUID of the joined bot

    ## Guidance for AI agents
    Call this after the bot has joined (status == "joined"). Recording does not
    start automatically on join.
    Example: `earl call --yes --json recall_ai.start_recording --bot_id <id>`
  EOT

  annotations {
    mode    = "write"
    secrets = ["recall_ai.api_key"]
  }

  param "bot_id" {
    type        = "string"
    required    = true
    description = "Bot UUID"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://api.recall.ai/api/v1/bot/{{ args.bot_id }}/start_recording/"

    auth {
      kind   = "bearer"
      secret = "recall_ai.api_key"
    }
  }

  result {
    output = "Recording started for bot {{ args.bot_id }}."
  }
}

command "stop_recording" {
  title       = "Stop recording"
  summary     = "Stop recording and begin transcript processing"
  description = <<-EOT
    Stops the active recording for a bot. Triggers transcript and media processing.
    After stopping, poll get_bot until media_shortcuts.transcript.status.code == "done"
    before retrieving the transcript.

    Parameters:
    - bot_id: UUID of the recording bot

    ## Guidance for AI agents
    Call this when the meeting ends or when you want to stop recording early.
    After stopping, the transcript will be processed asynchronously — poll get_bot
    for status before calling get_transcript.
    Example: `earl call --yes --json recall_ai.stop_recording --bot_id <id>`
  EOT

  annotations {
    mode    = "write"
    secrets = ["recall_ai.api_key"]
  }

  param "bot_id" {
    type        = "string"
    required    = true
    description = "Bot UUID"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://api.recall.ai/api/v1/bot/{{ args.bot_id }}/stop_recording/"

    auth {
      kind   = "bearer"
      secret = "recall_ai.api_key"
    }
  }

  result {
    output = "Recording stopped for bot {{ args.bot_id }}. Transcript processing will begin shortly — poll get_bot until media_shortcuts.transcript.status.code == \"done\"."
  }
}

command "pause_recording" {
  title       = "Pause recording"
  summary     = "Temporarily pause an active recording"
  description = <<-EOT
    Pauses the active recording without ending the session. Use resume_recording
    to continue. Useful for compliance scenarios (skip sensitive segments).

    Parameters:
    - bot_id: UUID of the recording bot

    ## Guidance for AI agents
    Use this to pause recording during sensitive discussion. Follow with
    resume_recording to continue.
    Example: `earl call --yes --json recall_ai.pause_recording --bot_id <id>`
  EOT

  annotations {
    mode    = "write"
    secrets = ["recall_ai.api_key"]
  }

  param "bot_id" {
    type        = "string"
    required    = true
    description = "Bot UUID"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://api.recall.ai/api/v1/bot/{{ args.bot_id }}/pause_recording/"

    auth {
      kind   = "bearer"
      secret = "recall_ai.api_key"
    }
  }

  result {
    output = "Recording paused for bot {{ args.bot_id }}."
  }
}

command "resume_recording" {
  title       = "Resume recording"
  summary     = "Resume a paused recording"
  description = <<-EOT
    Resumes a previously paused recording session.

    Parameters:
    - bot_id: UUID of the paused bot

    ## Guidance for AI agents
    Call this after pause_recording to resume capturing audio/video.
    Example: `earl call --yes --json recall_ai.resume_recording --bot_id <id>`
  EOT

  annotations {
    mode    = "write"
    secrets = ["recall_ai.api_key"]
  }

  param "bot_id" {
    type        = "string"
    required    = true
    description = "Bot UUID"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://api.recall.ai/api/v1/bot/{{ args.bot_id }}/resume_recording/"

    auth {
      kind   = "bearer"
      secret = "recall_ai.api_key"
    }
  }

  result {
    output = "Recording resumed for bot {{ args.bot_id }}."
  }
}

command "leave_call" {
  title       = "Leave call"
  summary     = "Remove the bot from an active meeting"
  description = <<-EOT
    Instructs the bot to leave the meeting immediately. Recording will stop and
    transcript processing will begin. Use this to end a recording session early
    or remove the bot when it is no longer needed.

    Parameters:
    - bot_id: UUID of the bot in the meeting

    ## Guidance for AI agents
    Use this when you want the bot to exit the meeting. After calling leave_call,
    poll get_bot until status == "done" and then retrieve the transcript.
    Example: `earl call --yes --json recall_ai.leave_call --bot_id <id>`
  EOT

  annotations {
    mode    = "write"
    secrets = ["recall_ai.api_key"]
  }

  param "bot_id" {
    type        = "string"
    required    = true
    description = "Bot UUID"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://api.recall.ai/api/v1/bot/{{ args.bot_id }}/leave_call/"

    auth {
      kind   = "bearer"
      secret = "recall_ai.api_key"
    }
  }

  result {
    output = "Bot {{ args.bot_id }} is leaving the call. Poll get_bot until media_shortcuts.transcript.status.code == \"done\" before retrieving transcript."
  }
}

command "send_chat_message" {
  title       = "Send chat message"
  summary     = "Send a message to the meeting chat"
  description = <<-EOT
    Posts a chat message visible to all meeting participants. Only works while
    the bot is in an active meeting session.

    Parameters:
    - bot_id: UUID of the bot in the meeting
    - message: Text content to send

    ## Guidance for AI agents
    Use this to communicate with meeting participants during a live session,
    e.g. to share a note, ask a clarifying question, or post a summary.
    Example: `earl call --yes --json recall_ai.send_chat_message --bot_id <id> --message "Here are the action items so far: ..."`
  EOT

  annotations {
    mode    = "write"
    secrets = ["recall_ai.api_key"]
  }

  param "bot_id" {
    type        = "string"
    required    = true
    description = "Bot UUID"
  }

  param "message" {
    type        = "string"
    required    = true
    description = "Text message to send to the meeting chat"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://api.recall.ai/api/v1/bot/{{ args.bot_id }}/send_chat_message/"

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
        message = "{{ args.message }}"
      }
    }
  }

  result {
    output = "Chat message sent to meeting."
  }
}

command "get_transcript" {
  title       = "Get transcript"
  summary     = "Get transcript metadata and download URL"
  description = <<-EOT
    Retrieves transcript metadata including status and a pre-signed download URL.
    The download URL is a temporary S3 link — use download_transcript to fetch
    the actual text content.

    The transcript ID comes from media_shortcuts.transcript.id in the get_bot response.

    Parameters:
    - transcript_id: UUID of the transcript (from get_bot media_shortcuts.transcript.id)

    ## Guidance for AI agents
    Only call this after get_bot shows media_shortcuts.transcript.status.code == "done".
    The response contains data.download_url — pass that URL directly to
    download_transcript to get the full transcript text.
    Example: `earl call --yes --json recall_ai.get_transcript --transcript_id <id>`
  EOT

  annotations {
    mode    = "read"
    secrets = ["recall_ai.api_key"]
  }

  param "transcript_id" {
    type        = "string"
    required    = true
    description = "Transcript UUID (from get_bot response: media_shortcuts.transcript.id)"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.recall.ai/api/v1/transcript/{{ args.transcript_id }}/"

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
    output = "Transcript {{ result.id }}\nStatus: {{ result.status.code | default(result.status) | default('unknown') }}\nCreated: {{ result.created_at }}\n\nDownload URL (pass to download_transcript):\n{{ result.data.download_url if result.data else 'not ready yet — wait for status == done' }}"
  }
}

command "download_transcript" {
  title       = "Download transcript text"
  summary     = "Fetch full transcript text from a download URL"
  description = <<-EOT
    Downloads and returns the complete transcript text from a pre-signed S3 URL.
    The URL comes from the data.download_url field returned by get_transcript.

    The transcript is returned as speaker-attributed JSON segments, each with
    participant name and word-level timestamps.

    WARNING: Download URLs expire in approximately 5 hours. Retrieve and process
    the content before expiry.

    Parameters:
    - url: Pre-signed S3 download URL from get_transcript response

    ## Guidance for AI agents
    This is the final step to get readable transcript text. Pass the download_url
    from get_transcript directly to this command. The result contains the full
    transcript you can summarize, search, or extract action items from.
    Example: `earl call --yes --json recall_ai.download_transcript --url "https://s3..."`
  EOT

  annotations {
    mode    = "read"
    secrets = []
  }

  param "url" {
    type        = "string"
    required    = true
    description = "Pre-signed S3 download URL from get_transcript response (data.download_url)"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "{{ args.url }}"

    headers = {
      Accept = "application/json"
    }
  }

  result {
    decode = "json"
    output = "Transcript ({{ result | length }} speaker segment(s)):\n\n{% for segment in result %}[{{ segment.participant.name | default('Unknown') }}]\n{% for word in segment.words %}{{ word.text }} {% endfor %}\n\n{% endfor %}"
  }
}

command "get_video" {
  title       = "Get video download URL"
  summary     = "Get the mixed video recording download URL"
  description = <<-EOT
    Retrieves the download URL for the mixed video recording (MP4). The URL is
    a pre-signed S3 link that expires in approximately 5 hours.

    The video ID comes from media_shortcuts.video_mixed.id in the get_bot response.

    Parameters:
    - video_id: UUID of the video_mixed artifact (from get_bot media_shortcuts.video_mixed.id)

    ## Guidance for AI agents
    Only call this when media_shortcuts.video_mixed.status.code == "done".
    Present the download URL to the user as a link — do NOT attempt to download
    or process binary video content.
    Example: `earl call --yes --json recall_ai.get_video --video_id <id>`
  EOT

  annotations {
    mode    = "read"
    secrets = ["recall_ai.api_key"]
  }

  param "video_id" {
    type        = "string"
    required    = true
    description = "video_mixed artifact UUID (from get_bot response: media_shortcuts.video_mixed.id)"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.recall.ai/api/v1/video_mixed/{{ args.video_id }}/"

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
    output = "Video recording ({{ result.format | default('mp4') }}) — status: {{ result.status.code | default(result.status) | default('unknown') }}\n\nDownload link (expires ~5 hours, share with user):\n{{ result.data.download_url if result.data else 'not ready yet — wait for status == done' }}"
  }
}

command "get_audio" {
  title       = "Get audio download URL"
  summary     = "Get the mixed audio recording download URL"
  description = <<-EOT
    Retrieves the download URL for the mixed audio recording (MP3 or raw).
    The URL is a pre-signed S3 link that expires in approximately 5 hours.

    The audio ID comes from media_shortcuts.audio_mixed.id in the get_bot response.

    Parameters:
    - audio_id: UUID of the audio_mixed artifact (from get_bot media_shortcuts.audio_mixed.id)

    ## Guidance for AI agents
    Only call this when media_shortcuts.audio_mixed.status.code == "done".
    Present the download URL to the user as a link — do NOT attempt to download
    or process binary audio content.
    Example: `earl call --yes --json recall_ai.get_audio --audio_id <id>`
  EOT

  annotations {
    mode    = "read"
    secrets = ["recall_ai.api_key"]
  }

  param "audio_id" {
    type        = "string"
    required    = true
    description = "audio_mixed artifact UUID (from get_bot response: media_shortcuts.audio_mixed.id)"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.recall.ai/api/v1/audio_mixed/{{ args.audio_id }}/"

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
    output = "Audio recording ({{ result.format | default('mp3') }}) — status: {{ result.status.code | default(result.status) | default('unknown') }}\n\nDownload link (expires ~5 hours, share with user):\n{{ result.data.download_url if result.data else 'not ready yet — wait for status == done' }}"
  }
}
