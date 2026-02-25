version = 1
provider = "openai"
categories = ["ai", "llm", "machine-learning"]

command "create_chat_completion" {
  title       = "Create chat completion"
  summary     = "Generate a chat completion using an OpenAI model"
  description = "Send a conversation to an OpenAI model and receive a generated response. Supports GPT-4o, GPT-4.1, o3, o4-mini, and other chat models."
  categories  = ["chat", "completions"]

  annotations {
    mode    = "write"
    secrets = ["openai.api_key"]
  }

  param "model" {
    type        = "string"
    required    = true
    description = "Model ID (e.g. gpt-4o, gpt-4o-mini, gpt-4.1, o3, o4-mini)"
  }

  param "system_prompt" {
    type        = "string"
    required    = false
    default     = ""
    # Kept as "" rather than removing: null content in the messages array is
    # rejected by the OpenAI API, so we must send "" when omitted.
    description = "System message to set the assistant's behavior"
  }

  param "message" {
    type        = "string"
    required    = true
    description = "User message content"
  }

  param "temperature" {
    type        = "number"
    required    = false
    default     = 1
    description = "Sampling temperature (0-2)"
  }

  param "max_completion_tokens" {
    type        = "integer"
    required    = false
    default     = 4096
    description = "Maximum number of tokens to generate (default 4096 is conservative; newer models support 16k+)"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://api.openai.com/v1/chat/completions"

    auth {
      kind   = "bearer"
      secret = "openai.api_key"
    }

    headers = {
      Content-Type = "application/json"
    }

    body {
      kind = "json"
      value = {
        model    = "{{ args.model }}"
        messages = [
          {
            role    = "system"
            content = "{{ args.system_prompt }}"
          },
          {
            role    = "user"
            content = "{{ args.message }}"
          }
        ]
        temperature          = "{{ args.temperature }}"
        max_completion_tokens = "{{ args.max_completion_tokens }}"
      }
    }
  }

  result {
    decode = "json"
    output = "Model: {{ result.model }}\nResponse: {{ result.choices[0].message.content }}\nFinish reason: {{ result.choices[0].finish_reason }}\nTokens: {{ result.usage.prompt_tokens }} prompt + {{ result.usage.completion_tokens }} completion = {{ result.usage.total_tokens }} total"
  }
}

command "list_models" {
  title       = "List models"
  summary     = "List all available OpenAI models"
  description = "Retrieve a list of all models currently available through the OpenAI API."
  categories  = ["models"]

  annotations {
    mode    = "read"
    secrets = ["openai.api_key"]
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.openai.com/v1/models"

    auth {
      kind   = "bearer"
      secret = "openai.api_key"
    }
  }

  result {
    decode = "json"
    output = "{% for model in result.data %}- {{ model.id }} (owned by {{ model.owned_by }})\n{% endfor %}"
  }
}

command "get_model" {
  title       = "Get model"
  summary     = "Retrieve details about a specific model"
  description = "Fetch metadata for a specific OpenAI model by its ID."
  categories  = ["models"]

  annotations {
    mode    = "read"
    secrets = ["openai.api_key"]
  }

  param "model" {
    type        = "string"
    required    = true
    description = "Model ID (e.g. gpt-4o, gpt-4.1)"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.openai.com/v1/models/{{ args.model }}"

    auth {
      kind   = "bearer"
      secret = "openai.api_key"
    }
  }

  result {
    decode = "json"
    output = "Model: {{ result.id }}\nOwner: {{ result.owned_by }}\nCreated: {{ result.created }}"
  }
}

command "create_embedding" {
  title       = "Create embedding"
  summary     = "Generate vector embeddings for text input"
  description = "Create vector embeddings for the given text input using an OpenAI embedding model."
  categories  = ["embeddings"]

  annotations {
    mode    = "write"
    secrets = ["openai.api_key"]
  }

  param "model" {
    type        = "string"
    required    = false
    default     = "text-embedding-3-small"
    description = "Embedding model (text-embedding-3-small, text-embedding-3-large, text-embedding-ada-002)"
  }

  param "input" {
    type        = "string"
    required    = true
    description = "Text to embed"
  }

  param "dimensions" {
    type        = "integer"
    required    = false
    description = "Output vector dimensions (embedding-3 models only)"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://api.openai.com/v1/embeddings"

    auth {
      kind   = "bearer"
      secret = "openai.api_key"
    }

    headers = {
      Content-Type = "application/json"
    }

    body {
      kind = "json"
      value = {
        model      = "{{ args.model }}"
        input      = "{{ args.input }}"
        dimensions = "{{ args.dimensions }}"
      }
    }
  }

  result {
    decode = "json"
    output = "Model: {{ result.model }}\nEmbeddings: {{ result.data | length }} vector(s)\nDimensions: {{ result.data[0].embedding | length }}\nTokens used: {{ result.usage.total_tokens }}"
  }
}

command "create_image" {
  title       = "Create image"
  summary     = "Generate an image from a text prompt"
  description = "Generate one or more images from a text description using DALL-E or GPT Image models."
  categories  = ["images"]

  annotations {
    mode    = "write"
    secrets = ["openai.api_key"]
  }

  param "prompt" {
    type        = "string"
    required    = true
    description = "Text description of the desired image"
  }

  param "model" {
    type        = "string"
    required    = false
    default     = "gpt-image-1"
    description = "Image model (gpt-image-1, dall-e-3, dall-e-2)"
  }

  param "size" {
    type        = "string"
    required    = false
    default     = "1024x1024"
    description = "Image dimensions (e.g. 1024x1024, 1792x1024, 1024x1792)"
  }

  param "quality" {
    type        = "string"
    required    = false
    default     = "auto"
    description = "Image quality (auto, low, medium, high for gpt-image; standard, hd for dall-e-3)"
  }

  param "n" {
    type        = "integer"
    required    = false
    default     = 1
    description = "Number of images to generate"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://api.openai.com/v1/images/generations"

    auth {
      kind   = "bearer"
      secret = "openai.api_key"
    }

    headers = {
      Content-Type = "application/json"
    }

    body {
      kind = "json"
      value = {
        prompt  = "{{ args.prompt }}"
        model   = "{{ args.model }}"
        size    = "{{ args.size }}"
        quality = "{{ args.quality }}"
        n       = "{{ args.n }}"
      }
    }
  }

  result {
    decode = "json"
    output = "Generated {{ result.data | length }} image(s)\n{% for img in result.data %}{{ img.url }}\n{% endfor %}"
  }
}

command "create_moderation" {
  title       = "Create moderation"
  summary     = "Check text for policy violations"
  description = "Classify text content for potential policy violations using OpenAI's moderation endpoint. This endpoint is free to use."
  categories  = ["moderation"]

  annotations {
    mode    = "read"
    secrets = ["openai.api_key"]
  }

  param "input" {
    type        = "string"
    required    = true
    description = "Text content to classify"
  }

  param "model" {
    type        = "string"
    required    = false
    default     = "omni-moderation-latest"
    description = "Moderation model (omni-moderation-latest, text-moderation-latest)"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://api.openai.com/v1/moderations"

    auth {
      kind   = "bearer"
      secret = "openai.api_key"
    }

    headers = {
      Content-Type = "application/json"
    }

    body {
      kind = "json"
      value = {
        input = "{{ args.input }}"
        model = "{{ args.model }}"
      }
    }
  }

  result {
    decode = "json"
    output = "Flagged: {{ result.results[0].flagged }}\nModel: {{ result.model }}"
  }
}

command "list_files" {
  title       = "List files"
  summary     = "List files uploaded to OpenAI"
  description = "Retrieve a list of files that have been uploaded to the OpenAI API, optionally filtered by purpose."
  categories  = ["files"]

  annotations {
    mode    = "read"
    secrets = ["openai.api_key"]
  }

  param "purpose" {
    type        = "string"
    required    = false
    description = "Filter by purpose (assistants, batch, fine-tune, vision, user_data)"
  }

  param "limit" {
    type        = "integer"
    required    = false
    default     = 20
    description = "Maximum number of files to return"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.openai.com/v1/files"

    auth {
      kind   = "bearer"
      secret = "openai.api_key"
    }

    query = {
      purpose = "{{ args.purpose }}"
      limit   = "{{ args.limit }}"
    }
  }

  result {
    decode = "json"
    output = "{% for file in result.data %}- {{ file.id }}: {{ file.filename }} ({{ file.bytes }} bytes, purpose: {{ file.purpose }})\n{% endfor %}Total: {{ result.data | length }} file(s)"
  }
}

command "delete_file" {
  title       = "Delete file"
  summary     = "Delete an uploaded file"
  description = "Delete a file that was previously uploaded to the OpenAI API."
  categories  = ["files"]

  annotations {
    mode    = "write"
    secrets = ["openai.api_key"]
  }

  param "file_id" {
    type        = "string"
    required    = true
    description = "File ID to delete"
  }

  operation {
    protocol = "http"
    method   = "DELETE"
    url      = "https://api.openai.com/v1/files/{{ args.file_id }}"

    auth {
      kind   = "bearer"
      secret = "openai.api_key"
    }
  }

  result {
    decode = "json"
    output = "Deleted: {{ result.id }} (success: {{ result.deleted }})"
  }
}

command "create_fine_tuning_job" {
  title       = "Create fine-tuning job"
  summary     = "Start a model fine-tuning job"
  description = "Create a fine-tuning job to customize a model with your training data."
  categories  = ["fine-tuning"]

  annotations {
    mode    = "write"
    secrets = ["openai.api_key"]
  }

  param "model" {
    type        = "string"
    required    = true
    description = "Base model to fine-tune (e.g. gpt-4o-mini-2024-07-18, gpt-4.1-2025-04-14)"
  }

  param "training_file" {
    type        = "string"
    required    = true
    description = "File ID of the uploaded JSONL training data"
  }

  param "validation_file" {
    type        = "string"
    required    = false
    description = "File ID for validation data"
  }

  param "suffix" {
    type        = "string"
    required    = false
    description = "Custom suffix for the fine-tuned model name (max 64 chars)"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://api.openai.com/v1/fine_tuning/jobs"

    auth {
      kind   = "bearer"
      secret = "openai.api_key"
    }

    headers = {
      Content-Type = "application/json"
    }

    body {
      kind = "json"
      value = {
        model           = "{{ args.model }}"
        training_file   = "{{ args.training_file }}"
        validation_file = "{{ args.validation_file }}"
        suffix          = "{{ args.suffix }}"
      }
    }
  }

  result {
    decode = "json"
    output = "Job: {{ result.id }}\nStatus: {{ result.status }}\nModel: {{ result.model }}\nTraining file: {{ result.training_file }}"
  }
}

command "list_fine_tuning_jobs" {
  title       = "List fine-tuning jobs"
  summary     = "List all fine-tuning jobs"
  description = "Retrieve a list of fine-tuning jobs for your organization."
  categories  = ["fine-tuning"]

  annotations {
    mode    = "read"
    secrets = ["openai.api_key"]
  }

  param "limit" {
    type        = "integer"
    required    = false
    default     = 20
    description = "Maximum number of jobs to return (1-100)"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.openai.com/v1/fine_tuning/jobs"

    auth {
      kind   = "bearer"
      secret = "openai.api_key"
    }

    query = {
      limit = "{{ args.limit }}"
    }
  }

  result {
    decode = "json"
    output = "{% for job in result.data %}- {{ job.id }}: {{ job.model }} ({{ job.status }})\n{% endfor %}"
  }
}
