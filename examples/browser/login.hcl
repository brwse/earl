version  = 1
provider = "browser"
categories = ["browser", "auth"]

command "login" {
  title       = "Login"
  summary     = "Log into a site using a form"
  description = "Fills email and password fields and submits the login form."

  annotations {
    mode    = "write"
    secrets = ["site.password"]
  }

  param "url" {
    type        = "string"
    required    = true
    description = "Login page URL"
  }

  param "email" {
    type        = "string"
    required    = true
    description = "Email address"
  }

  param "email_selector" {
    type        = "string"
    required    = false
    default     = "input[type=email]"
    description = "CSS selector for email field"
  }

  param "password_selector" {
    type        = "string"
    required    = false
    default     = "input[type=password]"
    description = "CSS selector for password field"
  }

  param "submit_selector" {
    type        = "string"
    required    = false
    default     = "button[type=submit]"
    description = "CSS selector for submit button"
  }

  param "session_id" {
    type        = "string"
    required    = false
    description = "Session ID to persist the logged-in browser"
  }

  operation {
    protocol = "browser"
    browser {
      session_id = "{{ args.session_id }}"
      headless   = true
      steps = [
        { action = "navigate",   url      = "{{ args.url }}" },
        { action = "fill",       selector = "{{ args.email_selector }}",    text = "{{ args.email }}" },
        { action = "fill",       selector = "{{ args.password_selector }}", text = "{{ secrets['site.password'] }}" },
        { action = "click",      selector = "{{ args.submit_selector }}" },
        { action = "wait_for",   timeout_ms = 5000, text = "" },
        { action = "snapshot" },
      ]
    }
  }

  result {
    decode = "json"
    output = "Logged in. Page state:\n{{ result.text | truncate(500) }}"
  }
}
