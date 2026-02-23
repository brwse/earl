version = 1
provider = "slack"
categories = ["messaging", "collaboration"]

command "send_message" {
  title       = "Send message"
  summary     = "Post a message to a Slack channel or DM"
  description = "Send a message to a channel, DM, or group conversation using the Slack chat.postMessage API."
  categories  = ["messaging"]

  annotations {
    mode    = "write"
    secrets = ["slack.bot_token"]
  }

  param "channel" {
    type        = "string"
    required    = true
    description = "Channel ID, DM ID, or user ID to post to"
  }

  param "text" {
    type        = "string"
    required    = true
    description = "Message body text"
  }

  param "thread_ts" {
    type        = "string"
    required    = false
    description = "Parent message timestamp to reply in a thread"
  }

  param "unfurl_links" {
    type        = "boolean"
    required    = false
    default     = true
    description = "Enable link previews"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://slack.com/api/chat.postMessage"

    auth {
      kind   = "bearer"
      secret = "slack.bot_token"
    }

    headers = {
      Content-Type = "application/json; charset=utf-8"
    }

    body {
      kind = "json"
      value = {
        channel      = "{{ args.channel }}"
        text         = "{{ args.text }}"
        thread_ts    = "{{ args.thread_ts }}"
        unfurl_links = "{{ args.unfurl_links }}"
      }
    }
  }

  result {
    decode = "json"
    output = "Message sent to {{ result.channel }} at ts={{ result.ts }}"
  }
}

command "update_message" {
  title       = "Update message"
  summary     = "Edit an existing Slack message"
  description = "Update the text or blocks of a previously sent message using chat.update."
  categories  = ["messaging"]

  annotations {
    mode    = "write"
    secrets = ["slack.bot_token"]
  }

  param "channel" {
    type        = "string"
    required    = true
    description = "Channel containing the message"
  }

  param "ts" {
    type        = "string"
    required    = true
    description = "Timestamp of the message to update"
  }

  param "text" {
    type        = "string"
    required    = true
    description = "New message text"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://slack.com/api/chat.update"

    auth {
      kind   = "bearer"
      secret = "slack.bot_token"
    }

    headers = {
      Content-Type = "application/json; charset=utf-8"
    }

    body {
      kind = "json"
      value = {
        channel = "{{ args.channel }}"
        ts      = "{{ args.ts }}"
        text    = "{{ args.text }}"
      }
    }
  }

  result {
    decode = "json"
    output = "Message updated in {{ result.channel }} at ts={{ result.ts }}"
  }
}

command "delete_message" {
  title       = "Delete message"
  summary     = "Delete a message from a Slack channel"
  description = "Delete a message from a conversation using chat.delete."
  categories  = ["messaging"]

  annotations {
    mode    = "write"
    secrets = ["slack.bot_token"]
  }

  param "channel" {
    type        = "string"
    required    = true
    description = "Channel containing the message"
  }

  param "ts" {
    type        = "string"
    required    = true
    description = "Timestamp of the message to delete"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://slack.com/api/chat.delete"

    auth {
      kind   = "bearer"
      secret = "slack.bot_token"
    }

    headers = {
      Content-Type = "application/json; charset=utf-8"
    }

    body {
      kind = "json"
      value = {
        channel = "{{ args.channel }}"
        ts      = "{{ args.ts }}"
      }
    }
  }

  result {
    decode = "json"
    output = "Message deleted from {{ result.channel }} at ts={{ result.ts }}"
  }
}

command "list_conversations" {
  title       = "List conversations"
  summary     = "List Slack channels and conversations"
  description = "List public and private channels, direct messages, and group conversations the bot has access to."
  categories  = ["channels"]

  annotations {
    mode    = "read"
    secrets = ["slack.bot_token"]
  }

  param "types" {
    type        = "string"
    required    = false
    default     = "public_channel"
    description = "Comma-separated types: public_channel, private_channel, mpim, im"
  }

  param "exclude_archived" {
    type        = "boolean"
    required    = false
    default     = false
    description = "Exclude archived channels from results"
  }

  param "limit" {
    type        = "integer"
    required    = false
    default     = 100
    description = "Maximum number of results per page (max 1000)"
  }

  param "cursor" {
    type        = "string"
    required    = false
    description = "Pagination cursor for next page of results"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://slack.com/api/conversations.list"

    auth {
      kind   = "bearer"
      secret = "slack.bot_token"
    }

    headers = {
      Content-Type = "application/json; charset=utf-8"
    }

    body {
      kind = "json"
      value = {
        types            = "{{ args.types }}"
        exclude_archived = "{{ args.exclude_archived }}"
        limit            = "{{ args.limit }}"
        cursor           = "{{ args.cursor }}"
      }
    }
  }

  result {
    decode = "json"
    output = "Found {{ result.channels | length }} channels{% for c in result.channels %}\n  #{{ c.name }} ({{ c.id }}) — {{ c.num_members }} members{% endfor %}"
  }
}

command "get_conversation_info" {
  title       = "Get conversation info"
  summary     = "Get details about a Slack channel or conversation"
  description = "Retrieve detailed information about a channel, DM, or group conversation."
  categories  = ["channels"]

  annotations {
    mode    = "read"
    secrets = ["slack.bot_token"]
  }

  param "channel" {
    type        = "string"
    required    = true
    description = "Conversation ID"
  }

  param "include_num_members" {
    type        = "boolean"
    required    = false
    default     = true
    description = "Include the member count in the response"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://slack.com/api/conversations.info"

    auth {
      kind   = "bearer"
      secret = "slack.bot_token"
    }

    headers = {
      Content-Type = "application/json; charset=utf-8"
    }

    body {
      kind = "json"
      value = {
        channel             = "{{ args.channel }}"
        include_num_members = "{{ args.include_num_members }}"
      }
    }
  }

  result {
    decode = "json"
    output = "#{{ result.channel.name }} ({{ result.channel.id }})\n  Topic: {{ result.channel.topic.value }}\n  Purpose: {{ result.channel.purpose.value }}\n  Members: {{ result.channel.num_members }}"
  }
}

command "get_conversation_history" {
  title       = "Get conversation history"
  summary     = "Fetch recent messages from a Slack channel"
  description = "Retrieve message history from a channel, DM, or group conversation."
  categories  = ["messaging"]

  annotations {
    mode    = "read"
    secrets = ["slack.bot_token"]
  }

  param "channel" {
    type        = "string"
    required    = true
    description = "Conversation ID"
  }

  param "limit" {
    type        = "integer"
    required    = false
    default     = 15
    description = "Maximum number of messages to return"
  }

  param "oldest" {
    type        = "string"
    required    = false
    description = "Only messages after this Unix timestamp"
  }

  param "latest" {
    type        = "string"
    required    = false
    description = "Only messages before this Unix timestamp"
  }

  param "cursor" {
    type        = "string"
    required    = false
    description = "Pagination cursor for next page of results"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://slack.com/api/conversations.history"

    auth {
      kind   = "bearer"
      secret = "slack.bot_token"
    }

    headers = {
      Content-Type = "application/json; charset=utf-8"
    }

    body {
      kind = "json"
      value = {
        channel = "{{ args.channel }}"
        limit   = "{{ args.limit }}"
        oldest  = "{{ args.oldest }}"
        latest  = "{{ args.latest }}"
        cursor  = "{{ args.cursor }}"
      }
    }
  }

  result {
    decode = "json"
    output = "{{ result.messages | length }} messages{% for m in result.messages %}\n  [{{ m.ts }}] {{ m.text | truncate(120) }}{% endfor %}{% if result.has_more %}\n  (more messages available){% endif %}"
  }
}

command "get_thread_replies" {
  title       = "Get thread replies"
  summary     = "Fetch replies in a message thread"
  description = "Retrieve all replies to a specific message thread using conversations.replies."
  categories  = ["messaging"]

  annotations {
    mode    = "read"
    secrets = ["slack.bot_token"]
  }

  param "channel" {
    type        = "string"
    required    = true
    description = "Conversation ID containing the thread"
  }

  param "ts" {
    type        = "string"
    required    = true
    description = "Parent message timestamp"
  }

  param "limit" {
    type        = "integer"
    required    = false
    default     = 15
    description = "Maximum number of replies to return"
  }

  param "cursor" {
    type        = "string"
    required    = false
    description = "Pagination cursor for next page of results"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://slack.com/api/conversations.replies"

    auth {
      kind   = "bearer"
      secret = "slack.bot_token"
    }

    headers = {
      Content-Type = "application/json; charset=utf-8"
    }

    body {
      kind = "json"
      value = {
        channel = "{{ args.channel }}"
        ts      = "{{ args.ts }}"
        limit   = "{{ args.limit }}"
        cursor  = "{{ args.cursor }}"
      }
    }
  }

  result {
    decode = "json"
    output = "{{ result.messages | length }} replies in thread{% for m in result.messages %}\n  [{{ m.ts }}] {{ m.text | truncate(120) }}{% endfor %}{% if result.has_more %}\n  (more replies available){% endif %}"
  }
}

command "create_conversation" {
  title       = "Create conversation"
  summary     = "Create a new Slack channel"
  description = "Create a new public or private channel in the workspace."
  categories  = ["channels"]

  annotations {
    mode    = "write"
    secrets = ["slack.bot_token"]
  }

  param "name" {
    type        = "string"
    required    = true
    description = "Channel name (lowercase, numbers, hyphens, underscores; max 80 chars)"
  }

  param "is_private" {
    type        = "boolean"
    required    = false
    default     = false
    description = "Create a private channel instead of public"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://slack.com/api/conversations.create"

    auth {
      kind   = "bearer"
      secret = "slack.bot_token"
    }

    headers = {
      Content-Type = "application/json; charset=utf-8"
    }

    body {
      kind = "json"
      value = {
        name       = "{{ args.name }}"
        is_private = "{{ args.is_private }}"
      }
    }
  }

  result {
    decode = "json"
    output = "Created channel #{{ result.channel.name }} ({{ result.channel.id }})"
  }
}

command "invite_to_conversation" {
  title       = "Invite to conversation"
  summary     = "Invite users to a Slack channel"
  description = "Invite one or more users to a channel by their user IDs."
  categories  = ["channels"]

  annotations {
    mode    = "write"
    secrets = ["slack.bot_token"]
  }

  param "channel" {
    type        = "string"
    required    = true
    description = "Channel ID to invite users to"
  }

  param "users" {
    type        = "string"
    required    = true
    description = "Comma-separated list of user IDs to invite"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://slack.com/api/conversations.invite"

    auth {
      kind   = "bearer"
      secret = "slack.bot_token"
    }

    headers = {
      Content-Type = "application/json; charset=utf-8"
    }

    body {
      kind = "json"
      value = {
        channel = "{{ args.channel }}"
        users   = "{{ args.users }}"
      }
    }
  }

  result {
    decode = "json"
    output = "Invited users to #{{ result.channel.name }} ({{ result.channel.id }})"
  }
}

command "archive_conversation" {
  title       = "Archive conversation"
  summary     = "Archive a Slack channel"
  description = "Archive a channel so it is read-only and hidden from the default channel list."
  categories  = ["channels"]

  annotations {
    mode    = "write"
    secrets = ["slack.bot_token"]
  }

  param "channel" {
    type        = "string"
    required    = true
    description = "Channel ID to archive"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://slack.com/api/conversations.archive"

    auth {
      kind   = "bearer"
      secret = "slack.bot_token"
    }

    headers = {
      Content-Type = "application/json; charset=utf-8"
    }

    body {
      kind = "json"
      value = {
        channel = "{{ args.channel }}"
      }
    }
  }

  result {
    decode = "json"
    output = "Channel archived successfully"
  }
}

command "set_conversation_topic" {
  title       = "Set conversation topic"
  summary     = "Set the topic of a Slack channel"
  description = "Update the topic text displayed at the top of a channel."
  categories  = ["channels"]

  annotations {
    mode    = "write"
    secrets = ["slack.bot_token"]
  }

  param "channel" {
    type        = "string"
    required    = true
    description = "Channel ID"
  }

  param "topic" {
    type        = "string"
    required    = true
    description = "New topic text for the channel"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://slack.com/api/conversations.setTopic"

    auth {
      kind   = "bearer"
      secret = "slack.bot_token"
    }

    headers = {
      Content-Type = "application/json; charset=utf-8"
    }

    body {
      kind = "json"
      value = {
        channel = "{{ args.channel }}"
        topic   = "{{ args.topic }}"
      }
    }
  }

  result {
    decode = "json"
    output = "Topic set for #{{ result.channel.name }}: {{ result.channel.topic.value }}"
  }
}

command "list_users" {
  title       = "List users"
  summary     = "List all users in the Slack workspace"
  description = "Retrieve a paginated list of all users in the workspace, including deactivated accounts."
  categories  = ["users"]

  annotations {
    mode    = "read"
    secrets = ["slack.bot_token"]
  }

  param "limit" {
    type        = "integer"
    required    = false
    default     = 100
    description = "Maximum number of results per page (max 1000)"
  }

  param "cursor" {
    type        = "string"
    required    = false
    description = "Pagination cursor for next page of results"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://slack.com/api/users.list"

    auth {
      kind   = "bearer"
      secret = "slack.bot_token"
    }

    headers = {
      Content-Type = "application/json; charset=utf-8"
    }

    body {
      kind = "json"
      value = {
        limit  = "{{ args.limit }}"
        cursor = "{{ args.cursor }}"
      }
    }
  }

  result {
    decode = "json"
    output = "{{ result.members | length }} users{% for u in result.members %}\n  {{ u.real_name }} (@{{ u.name }}, {{ u.id }}){% if u.deleted %} [deactivated]{% endif %}{% endfor %}"
  }
}

command "get_user_info" {
  title       = "Get user info"
  summary     = "Get detailed information about a Slack user"
  description = "Retrieve profile details for a specific user by their user ID."
  categories  = ["users"]

  annotations {
    mode    = "read"
    secrets = ["slack.bot_token"]
  }

  param "user" {
    type        = "string"
    required    = true
    description = "User ID to look up"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://slack.com/api/users.info"

    auth {
      kind   = "bearer"
      secret = "slack.bot_token"
    }

    headers = {
      Content-Type = "application/json; charset=utf-8"
    }

    body {
      kind = "json"
      value = {
        user = "{{ args.user }}"
      }
    }
  }

  result {
    decode = "json"
    output = "{{ result.user.real_name }} (@{{ result.user.name }})\n  Email: {{ result.user.profile.email }}\n  Title: {{ result.user.profile.title }}\n  Status: {{ result.user.profile.status_emoji }} {{ result.user.profile.status_text }}"
  }
}

command "lookup_user_by_email" {
  title       = "Lookup user by email"
  summary     = "Find a Slack user by their email address"
  description = "Look up a user's profile and ID using their email address."
  categories  = ["users"]

  annotations {
    mode    = "read"
    secrets = ["slack.bot_token"]
  }

  param "email" {
    type        = "string"
    required    = true
    description = "Email address to look up"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://slack.com/api/users.lookupByEmail"

    auth {
      kind   = "bearer"
      secret = "slack.bot_token"
    }

    headers = {
      Content-Type = "application/json; charset=utf-8"
    }

    body {
      kind = "json"
      value = {
        email = "{{ args.email }}"
      }
    }
  }

  result {
    decode = "json"
    output = "{{ result.user.real_name }} (@{{ result.user.name }}, {{ result.user.id }})\n  Email: {{ result.user.profile.email }}"
  }
}

command "search_messages" {
  title       = "Search messages"
  summary     = "Search for messages across Slack channels"
  description = "Search messages across the workspace. Supports modifiers like in:#channel, from:@user. Requires a user token (xoxp-), not a bot token."
  categories  = ["search"]

  annotations {
    mode    = "read"
    secrets = ["slack.user_token"]
  }

  param "query" {
    type        = "string"
    required    = true
    description = "Search query (supports modifiers like in:#channel, from:@user)"
  }

  param "count" {
    type        = "integer"
    required    = false
    default     = 20
    description = "Number of results per page (max 100)"
  }

  param "sort" {
    type        = "string"
    required    = false
    default     = "score"
    description = "Sort order: score or timestamp"
  }

  param "sort_dir" {
    type        = "string"
    required    = false
    default     = "desc"
    description = "Sort direction: asc or desc"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://slack.com/api/search.messages"

    auth {
      kind   = "bearer"
      secret = "slack.user_token"
    }

    headers = {
      Content-Type = "application/json; charset=utf-8"
    }

    body {
      kind = "json"
      value = {
        query    = "{{ args.query }}"
        count    = "{{ args.count }}"
        sort     = "{{ args.sort }}"
        sort_dir = "{{ args.sort_dir }}"
      }
    }
  }

  result {
    decode = "json"
    output = "{{ result.messages.total }} results for \"{{ args.query }}\"{% for m in result.messages.matches %}\n  [#{{ m.channel.name }}] {{ m.text | truncate(120) }}{% endfor %}"
  }
}
