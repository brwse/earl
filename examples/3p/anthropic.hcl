version = 1
provider = "anthropic"
categories = ["ai", "llm"]

command "create_message" {
  title       = "Create message"
  summary     = "Send a message to Claude and get a response"
  description = "Create a message using the Anthropic Messages API. Supports system prompts, temperature control, and multiple sampling parameters."
  categories  = ["write", "messages"]

  annotations {
    mode    = "write"
    secrets = ["anthropic.api_key"]
  }

  param "model" {
    type        = "string"
    required    = true
    description = "Model ID (e.g. claude-sonnet-4-6, claude-haiku-4-5-20251001)"
  }

  param "max_tokens" {
    type        = "integer"
    required    = true
    description = "Maximum number of output tokens"
  }

  param "messages" {
    type        = "string"
    required    = true
    description = "JSON array of message objects, e.g. [{\"role\":\"user\",\"content\":\"Hello\"}]"
  }

  param "system" {
    type        = "string"
    required    = false
    description = "System prompt to set context for the conversation"
  }

  param "temperature" {
    type        = "number"
    required    = false
    description = "Sampling temperature between 0.0 and 1.0"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://api.anthropic.com/v1/messages"

    auth {
      kind     = "api_key"
      location = "header"
      name     = "x-api-key"
      secret   = "anthropic.api_key"
    }

    headers = {
      anthropic-version = "2023-06-01"
    }

    body {
      kind = "json"
      value = {
        model      = "{{ args.model }}"
        max_tokens = "{{ args.max_tokens }}"
        messages   = "{{ args.messages }}"
        system     = "{{ args.system }}"
        temperature = "{{ args.temperature }}"
      }
    }
  }

  result {
    decode = "json"
    output = "Model: {{ result.model }}\nStop reason: {{ result.stop_reason }}\nContent: {{ result.content[0].text }}\nUsage: {{ result.usage.input_tokens }} in / {{ result.usage.output_tokens }} out"
  }
}

command "count_tokens" {
  title       = "Count tokens"
  summary     = "Count the tokens in a message without sending it"
  description = "Count the number of tokens in a Messages API request without creating a message. Useful for estimating costs and checking context limits."
  categories  = ["read", "messages"]

  annotations {
    mode    = "read"
    secrets = ["anthropic.api_key"]
  }

  param "model" {
    type        = "string"
    required    = true
    description = "Model ID to count tokens for"
  }

  param "messages" {
    type        = "string"
    required    = true
    description = "JSON array of message objects"
  }

  param "system" {
    type        = "string"
    required    = false
    description = "System prompt to include in token count"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://api.anthropic.com/v1/messages/count_tokens"

    auth {
      kind     = "api_key"
      location = "header"
      name     = "x-api-key"
      secret   = "anthropic.api_key"
    }

    headers = {
      anthropic-version = "2023-06-01"
    }

    body {
      kind = "json"
      value = {
        model    = "{{ args.model }}"
        messages = "{{ args.messages }}"
        system   = "{{ args.system }}"
      }
    }
  }

  result {
    decode = "json"
    output = "Input tokens: {{ result.input_tokens }}"
  }
}

command "list_models" {
  title       = "List models"
  summary     = "List all available Claude models"
  description = "List available models on the Anthropic API with pagination support."
  categories  = ["read", "models"]

  annotations {
    mode    = "read"
    secrets = ["anthropic.api_key"]
  }

  param "limit" {
    type        = "integer"
    required    = false
    default     = 20
    description = "Number of models to return (1-1000)"
  }

  param "after_id" {
    type        = "string"
    required    = false
    description = "Cursor for forward pagination"
  }

  param "before_id" {
    type        = "string"
    required    = false
    description = "Cursor for backward pagination"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.anthropic.com/v1/models"

    auth {
      kind     = "api_key"
      location = "header"
      name     = "x-api-key"
      secret   = "anthropic.api_key"
    }

    query = {
      limit     = "{{ args.limit }}"
      after_id  = "{{ args.after_id }}"
      before_id = "{{ args.before_id }}"
    }

    headers = {
      anthropic-version = "2023-06-01"
    }
  }

  result {
    decode = "json"
    output = "Models ({{ result.data | length }}):\n{% for m in result.data %}  - {{ m.id }} ({{ m.display_name }}, created {{ m.created_at }})\n{% endfor %}Has more: {{ result.has_more }}"
  }
}

command "get_model" {
  title       = "Get model"
  summary     = "Retrieve details about a specific model"
  description = "Get information about a specific Claude model by its ID."
  categories  = ["read", "models"]

  annotations {
    mode    = "read"
    secrets = ["anthropic.api_key"]
  }

  param "model_id" {
    type        = "string"
    required    = true
    description = "Model identifier (e.g. claude-sonnet-4-6)"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.anthropic.com/v1/models/{{ args.model_id }}"

    auth {
      kind     = "api_key"
      location = "header"
      name     = "x-api-key"
      secret   = "anthropic.api_key"
    }

    headers = {
      anthropic-version = "2023-06-01"
    }
  }

  result {
    decode = "json"
    output = "Model: {{ result.id }}\nDisplay name: {{ result.display_name }}\nCreated: {{ result.created_at }}"
  }
}

command "create_batch" {
  title       = "Create batch"
  summary     = "Create a batch of message requests for async processing"
  description = "Submit a batch of Messages API requests for asynchronous processing. Each request in the batch includes a custom_id and params matching the Messages API body."
  categories  = ["write", "batches"]

  annotations {
    mode    = "write"
    secrets = ["anthropic.api_key"]
  }

  param "requests" {
    type        = "string"
    required    = true
    description = "JSON array of batch request objects, each with custom_id and params"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://api.anthropic.com/v1/messages/batches"

    auth {
      kind     = "api_key"
      location = "header"
      name     = "x-api-key"
      secret   = "anthropic.api_key"
    }

    headers = {
      anthropic-version = "2023-06-01"
    }

    body {
      kind = "json"
      value = {
        requests = "{{ args.requests }}"
      }
    }
  }

  result {
    decode = "json"
    output = "Batch ID: {{ result.id }}\nStatus: {{ result.processing_status }}\nRequests: {{ result.request_counts.processing }} processing, {{ result.request_counts.succeeded }} succeeded, {{ result.request_counts.errored }} errored\nCreated: {{ result.created_at }}\nExpires: {{ result.expires_at }}"
  }
}

command "list_batches" {
  title       = "List batches"
  summary     = "List all message batches"
  description = "List message batches with pagination support."
  categories  = ["read", "batches"]

  annotations {
    mode    = "read"
    secrets = ["anthropic.api_key"]
  }

  param "limit" {
    type        = "integer"
    required    = false
    default     = 20
    description = "Number of batches to return (1-1000)"
  }

  param "after_id" {
    type        = "string"
    required    = false
    description = "Cursor for forward pagination"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.anthropic.com/v1/messages/batches"

    auth {
      kind     = "api_key"
      location = "header"
      name     = "x-api-key"
      secret   = "anthropic.api_key"
    }

    query = {
      limit    = "{{ args.limit }}"
      after_id = "{{ args.after_id }}"
    }

    headers = {
      anthropic-version = "2023-06-01"
    }
  }

  result {
    decode = "json"
    output = "Batches ({{ result.data | length }}):\n{% for b in result.data %}  - {{ b.id }}: {{ b.processing_status }}\n{% endfor %}Has more: {{ result.has_more }}"
  }
}

command "get_batch" {
  title       = "Get batch"
  summary     = "Retrieve details about a message batch"
  description = "Get the current status and details of a specific message batch."
  categories  = ["read", "batches"]

  annotations {
    mode    = "read"
    secrets = ["anthropic.api_key"]
  }

  param "batch_id" {
    type        = "string"
    required    = true
    description = "Message batch ID"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.anthropic.com/v1/messages/batches/{{ args.batch_id }}"

    auth {
      kind     = "api_key"
      location = "header"
      name     = "x-api-key"
      secret   = "anthropic.api_key"
    }

    headers = {
      anthropic-version = "2023-06-01"
    }
  }

  result {
    decode = "json"
    output = "Batch: {{ result.id }}\nStatus: {{ result.processing_status }}\nProcessing: {{ result.request_counts.processing }}\nSucceeded: {{ result.request_counts.succeeded }}\nErrored: {{ result.request_counts.errored }}\nCanceled: {{ result.request_counts.canceled }}\nCreated: {{ result.created_at }}"
  }
}

command "cancel_batch" {
  title       = "Cancel batch"
  summary     = "Cancel a running message batch"
  description = "Initiate cancellation of a message batch that is currently processing."
  categories  = ["write", "batches"]

  annotations {
    mode    = "write"
    secrets = ["anthropic.api_key"]
  }

  param "batch_id" {
    type        = "string"
    required    = true
    description = "Message batch ID to cancel"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://api.anthropic.com/v1/messages/batches/{{ args.batch_id }}/cancel"

    auth {
      kind     = "api_key"
      location = "header"
      name     = "x-api-key"
      secret   = "anthropic.api_key"
    }

    headers = {
      anthropic-version = "2023-06-01"
    }
  }

  result {
    decode = "json"
    output = "Batch {{ result.id }} cancellation initiated.\nStatus: {{ result.processing_status }}"
  }
}

command "delete_batch" {
  title       = "Delete batch"
  summary     = "Delete a completed message batch"
  description = "Delete a message batch that has reached an ended state (completed, canceled, or expired)."
  categories  = ["write", "batches"]

  annotations {
    mode    = "write"
    secrets = ["anthropic.api_key"]
  }

  param "batch_id" {
    type        = "string"
    required    = true
    description = "Message batch ID to delete (must be in ended state)"
  }

  operation {
    protocol = "http"
    method   = "DELETE"
    url      = "https://api.anthropic.com/v1/messages/batches/{{ args.batch_id }}"

    auth {
      kind     = "api_key"
      location = "header"
      name     = "x-api-key"
      secret   = "anthropic.api_key"
    }

    headers = {
      anthropic-version = "2023-06-01"
    }
  }

  result {
    decode = "json"
    output = "Deleted batch: {{ result.id }}"
  }
}

command "get_batch_results" {
  title       = "Get batch results"
  summary     = "Download results from a completed batch"
  description = "Retrieve the results of a completed message batch. Returns streamed JSONL with each line containing a custom_id and result."
  categories  = ["read", "batches"]

  annotations {
    mode    = "read"
    secrets = ["anthropic.api_key"]
  }

  param "batch_id" {
    type        = "string"
    required    = true
    description = "Message batch ID to get results for"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.anthropic.com/v1/messages/batches/{{ args.batch_id }}/results"

    auth {
      kind     = "api_key"
      location = "header"
      name     = "x-api-key"
      secret   = "anthropic.api_key"
    }

    headers = {
      anthropic-version = "2023-06-01"
    }
  }

  result {
    decode = "json"
    output = "Batch results for {{ args.batch_id }} (JSONL stream)"
  }
}

command "list_files" {
  title       = "List files"
  summary     = "List uploaded files"
  description = "List files uploaded to the Anthropic API (beta). Supports pagination."
  categories  = ["read", "files"]

  annotations {
    mode    = "read"
    secrets = ["anthropic.api_key"]
  }

  param "limit" {
    type        = "integer"
    required    = false
    default     = 20
    description = "Number of files to return (1-1000)"
  }

  param "after_id" {
    type        = "string"
    required    = false
    description = "Cursor for forward pagination"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.anthropic.com/v1/files"

    auth {
      kind     = "api_key"
      location = "header"
      name     = "x-api-key"
      secret   = "anthropic.api_key"
    }

    query = {
      limit    = "{{ args.limit }}"
      after_id = "{{ args.after_id }}"
    }

    headers = {
      anthropic-version = "2023-06-01"
      anthropic-beta    = "files-api-2025-04-14"
    }
  }

  result {
    decode = "json"
    output = "Files ({{ result.data | length }}):\n{% for f in result.data %}  - {{ f.id }}: {{ f.filename }} ({{ f.mime_type }}, {{ f.size_bytes }} bytes)\n{% endfor %}Has more: {{ result.has_more }}"
  }
}

command "delete_file" {
  title       = "Delete file"
  summary     = "Delete an uploaded file"
  description = "Delete a file that was previously uploaded to the Anthropic API (beta)."
  categories  = ["write", "files"]

  annotations {
    mode    = "write"
    secrets = ["anthropic.api_key"]
  }

  param "file_id" {
    type        = "string"
    required    = true
    description = "File ID to delete"
  }

  operation {
    protocol = "http"
    method   = "DELETE"
    url      = "https://api.anthropic.com/v1/files/{{ args.file_id }}"

    auth {
      kind     = "api_key"
      location = "header"
      name     = "x-api-key"
      secret   = "anthropic.api_key"
    }

    headers = {
      anthropic-version = "2023-06-01"
      anthropic-beta    = "files-api-2025-04-14"
    }
  }

  result {
    decode = "json"
    output = "Deleted file: {{ result.id }}"
  }
}
