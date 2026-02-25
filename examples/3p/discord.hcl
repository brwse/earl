version = 1
provider = "discord"
categories = ["messaging", "community"]

command "send_message" {
  title       = "Send message"
  summary     = "Send a message to a channel"
  description = "Send a text message to a Discord channel."
  categories  = ["messaging"]

  annotations {
    mode    = "write"
    secrets = ["discord.bot_token"]
  }

  param "channel_id" {
    type        = "string"
    required    = true
    description = "The channel ID to send the message to"
  }

  param "content" {
    type        = "string"
    required    = true
    description = "Message text content, max 2000 characters"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://discord.com/api/v10/channels/{{ args.channel_id }}/messages"

    auth {
      kind   = "bearer"
      secret = "discord.bot_token"
    }

    body {
      kind = "json"
      value = {
        content = "{{ args.content }}"
      }
    }
  }

  result {
    decode = "json"
    output = "Sent message {{ result.id }} in channel {{ result.channel_id }} by {{ result.author.username }}"
  }
}

command "list_messages" {
  title       = "List messages"
  summary     = "List recent messages in a channel"
  description = "Retrieve recent messages from a Discord channel, ordered by most recent first."
  categories  = ["messaging"]

  annotations {
    mode    = "read"
    secrets = ["discord.bot_token"]
  }

  param "channel_id" {
    type        = "string"
    required    = true
    description = "The channel ID to fetch messages from"
  }

  param "limit" {
    type        = "integer"
    required    = false
    default     = 50
    description = "Number of messages to return (1-100)"
  }

  param "before" {
    type        = "string"
    required    = false
    description = "Get messages before this message ID"
  }

  param "after" {
    type        = "string"
    required    = false
    description = "Get messages after this message ID"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://discord.com/api/v10/channels/{{ args.channel_id }}/messages"

    auth {
      kind   = "bearer"
      secret = "discord.bot_token"
    }

    query = {
      limit  = "{{ args.limit }}"
      before = "{{ args.before }}"
      after  = "{{ args.after }}"
    }
  }

  result {
    decode = "json"
    output = "{{ result | length }} messages in channel {{ args.channel_id }}"
  }
}

command "get_message" {
  title       = "Get message"
  summary     = "Get a specific message by ID"
  description = "Retrieve a single message from a Discord channel by its message ID."
  categories  = ["messaging"]

  annotations {
    mode    = "read"
    secrets = ["discord.bot_token"]
  }

  param "channel_id" {
    type        = "string"
    required    = true
    description = "The channel ID containing the message"
  }

  param "message_id" {
    type        = "string"
    required    = true
    description = "The message ID to retrieve"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://discord.com/api/v10/channels/{{ args.channel_id }}/messages/{{ args.message_id }}"

    auth {
      kind   = "bearer"
      secret = "discord.bot_token"
    }
  }

  result {
    decode = "json"
    output = "Message {{ result.id }} by {{ result.author.username }}: {{ result.content }}"
  }
}

command "edit_message" {
  title       = "Edit message"
  summary     = "Edit an existing message"
  description = "Edit a message previously sent by the bot in a Discord channel."
  categories  = ["messaging"]

  annotations {
    mode    = "write"
    secrets = ["discord.bot_token"]
  }

  param "channel_id" {
    type        = "string"
    required    = true
    description = "The channel ID containing the message"
  }

  param "message_id" {
    type        = "string"
    required    = true
    description = "The message ID to edit"
  }

  param "content" {
    type        = "string"
    required    = true
    description = "New message text content, max 2000 characters"
  }

  operation {
    protocol = "http"
    method   = "PATCH"
    url      = "https://discord.com/api/v10/channels/{{ args.channel_id }}/messages/{{ args.message_id }}"

    auth {
      kind   = "bearer"
      secret = "discord.bot_token"
    }

    body {
      kind = "json"
      value = {
        content = "{{ args.content }}"
      }
    }
  }

  result {
    decode = "json"
    output = "Updated message {{ result.id }}, edited at {{ result.edited_timestamp }}"
  }
}

command "delete_message" {
  title       = "Delete message"
  summary     = "Delete a message from a channel"
  description = "Delete a specific message from a Discord channel. Bot must have Manage Messages permission to delete others' messages."
  categories  = ["messaging"]

  annotations {
    mode    = "write"
    secrets = ["discord.bot_token"]
  }

  param "channel_id" {
    type        = "string"
    required    = true
    description = "The channel ID containing the message"
  }

  param "message_id" {
    type        = "string"
    required    = true
    description = "The message ID to delete"
  }

  operation {
    protocol = "http"
    method   = "DELETE"
    url      = "https://discord.com/api/v10/channels/{{ args.channel_id }}/messages/{{ args.message_id }}"

    auth {
      kind   = "bearer"
      secret = "discord.bot_token"
    }
  }

  result {
    decode = "json"
    output = "Deleted message {{ args.message_id }} from channel {{ args.channel_id }}"
  }
}

command "get_channel" {
  title       = "Get channel"
  summary     = "Get channel details by ID"
  description = "Retrieve details about a Discord channel including its name, type, topic, and guild."
  categories  = ["community"]

  annotations {
    mode    = "read"
    secrets = ["discord.bot_token"]
  }

  param "channel_id" {
    type        = "string"
    required    = true
    description = "The channel ID to look up"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://discord.com/api/v10/channels/{{ args.channel_id }}"

    auth {
      kind   = "bearer"
      secret = "discord.bot_token"
    }
  }

  result {
    decode = "json"
    output = "Channel: {{ result.name }} ({{ result.id }}) type={{ result.type }} guild={{ result.guild_id }}"
  }
}

command "list_guild_channels" {
  title       = "List guild channels"
  summary     = "List all channels in a guild"
  description = "Retrieve all channels in a Discord guild/server. Includes text, voice, category, and other channel types."
  categories  = ["community"]

  annotations {
    mode    = "read"
    secrets = ["discord.bot_token"]
  }

  param "guild_id" {
    type        = "string"
    required    = true
    description = "The guild/server ID"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://discord.com/api/v10/guilds/{{ args.guild_id }}/channels"

    auth {
      kind   = "bearer"
      secret = "discord.bot_token"
    }
  }

  result {
    decode = "json"
    output = "{{ result | length }} channels in guild {{ args.guild_id }}"
  }
}

command "create_channel" {
  title       = "Create channel"
  summary     = "Create a new channel in a guild"
  description = "Create a new text, voice, or category channel in a Discord guild. Requires Manage Channels permission."
  categories  = ["community"]

  annotations {
    mode    = "write"
    secrets = ["discord.bot_token"]
  }

  param "guild_id" {
    type        = "string"
    required    = true
    description = "The guild/server ID"
  }

  param "name" {
    type        = "string"
    required    = true
    description = "Channel name, 1-100 characters"
  }

  param "type" {
    type        = "integer"
    required    = false
    default     = 0
    description = "Channel type (0=text, 2=voice, 4=category, 5=announcement, 13=stage, 15=forum)"
  }

  param "topic" {
    type        = "string"
    required    = false
    description = "Channel topic, 0-1024 characters"
  }

  param "parent_id" {
    type        = "string"
    required    = false
    description = "Parent category ID to nest this channel under"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://discord.com/api/v10/guilds/{{ args.guild_id }}/channels"

    auth {
      kind   = "bearer"
      secret = "discord.bot_token"
    }

    body {
      kind = "json"
      value = {
        name      = "{{ args.name }}"
        type      = "{{ args.type }}"
        topic     = "{{ args.topic }}"
        parent_id = "{{ args.parent_id }}"
      }
    }
  }

  result {
    decode = "json"
    output = "Created channel {{ result.name }} ({{ result.id }}) in guild {{ result.guild_id }}"
  }
}

command "get_guild" {
  title       = "Get guild"
  summary     = "Get guild/server details by ID"
  description = "Retrieve details about a Discord guild including name, owner, member count, and features."
  categories  = ["community"]

  annotations {
    mode    = "read"
    secrets = ["discord.bot_token"]
  }

  param "guild_id" {
    type        = "string"
    required    = true
    description = "The guild/server ID"
  }

  param "with_counts" {
    type        = "boolean"
    required    = false
    default     = true
    description = "Include approximate member and presence counts"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://discord.com/api/v10/guilds/{{ args.guild_id }}"

    auth {
      kind   = "bearer"
      secret = "discord.bot_token"
    }

    query = {
      with_counts = "{{ args.with_counts }}"
    }
  }

  result {
    decode = "json"
    output = "Guild: {{ result.name }} ({{ result.id }}) owner={{ result.owner_id }} members={{ result.approximate_member_count }}"
  }
}

command "list_guilds" {
  title       = "List guilds"
  summary     = "List guilds the bot is a member of"
  description = "Retrieve all guilds/servers the bot has joined."
  categories  = ["community"]

  annotations {
    mode    = "read"
    secrets = ["discord.bot_token"]
  }

  param "limit" {
    type        = "integer"
    required    = false
    default     = 200
    description = "Max number of guilds to return (1-200)"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://discord.com/api/v10/users/@me/guilds"

    auth {
      kind   = "bearer"
      secret = "discord.bot_token"
    }

    query = {
      limit = "{{ args.limit }}"
    }
  }

  result {
    decode = "json"
    output = "Bot is in {{ result | length }} guilds"
  }
}

command "add_reaction" {
  title       = "Add reaction"
  summary     = "Add a reaction emoji to a message"
  description = "Add a reaction to a message. Use URL-encoded Unicode emoji (e.g. %F0%9F%91%8D for thumbsup) or custom emoji in name:id format."
  categories  = ["messaging"]

  annotations {
    mode    = "write"
    secrets = ["discord.bot_token"]
  }

  param "channel_id" {
    type        = "string"
    required    = true
    description = "The channel ID containing the message"
  }

  param "message_id" {
    type        = "string"
    required    = true
    description = "The message ID to react to"
  }

  param "emoji" {
    type        = "string"
    required    = true
    description = "URL-encoded emoji (e.g. %F0%9F%91%8D) or custom emoji as name:id"
  }

  operation {
    protocol = "http"
    method   = "PUT"
    url      = "https://discord.com/api/v10/channels/{{ args.channel_id }}/messages/{{ args.message_id }}/reactions/{{ args.emoji }}/@me"

    auth {
      kind   = "bearer"
      secret = "discord.bot_token"
    }
  }

  result {
    decode = "json"
    output = "Added reaction {{ args.emoji }} to message {{ args.message_id }}"
  }
}

command "get_current_user" {
  title       = "Get current user"
  summary     = "Get the bot's own user profile"
  description = "Retrieve the user object for the currently authenticated bot account."
  categories  = ["community"]

  annotations {
    mode    = "read"
    secrets = ["discord.bot_token"]
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://discord.com/api/v10/users/@me"

    auth {
      kind   = "bearer"
      secret = "discord.bot_token"
    }
  }

  result {
    decode = "json"
    output = "Bot: {{ result.username }}#{{ result.discriminator }} ({{ result.id }}) verified={{ result.verified }}"
  }
}
