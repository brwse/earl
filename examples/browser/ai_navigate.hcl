version  = 1
provider = "browser"
categories = ["browser", "navigation"]

command "snapshot" {
  title       = "Snapshot"
  summary     = "Get an accessibility tree snapshot of the current browser session"
  description = "Returns the accessibility tree of the current page in a persistent browser session. Use the returned refs to click, fill, or interact with elements in subsequent commands."

  annotations {
    mode = "read"
  }

  param "session_id" {
    type        = "string"
    required    = true
    description = "Browser session ID"
  }

  operation {
    protocol = "browser"
    browser {
      session_id = "{{ args.session_id }}"
      steps = [{ action = "snapshot" }]
    }
  }

  result {
    decode = "json"
    output = "{{ result.text }}"
  }
}

command "click" {
  title       = "Click element"
  summary     = "Click an element in a persistent browser session by ref or CSS selector"
  description = "Click an element identified by a ref (from a snapshot) or a CSS selector."

  annotations {
    mode = "write"
  }

  param "session_id" {
    type        = "string"
    required    = true
    description = "Browser session ID"
  }

  param "ref" {
    type        = "string"
    required    = false
    description = "Accessibility ref from a prior snapshot (e.g. e8)"
  }

  param "selector" {
    type        = "string"
    required    = false
    description = "CSS selector (alternative to ref)"
  }

  operation {
    protocol = "browser"
    browser {
      session_id = "{{ args.session_id }}"
      steps = [
        { action = "click", ref = "{{ args.ref }}", selector = "{{ args.selector }}" },
        { action = "snapshot" },
      ]
    }
  }

  result {
    decode = "json"
    output = "Clicked. Updated page state:\n{{ result.text | truncate(500) }}"
  }
}

command "navigate" {
  title       = "Navigate"
  summary     = "Navigate to a URL in a persistent browser session"
  description = "Navigate the browser session to a URL and return the page snapshot."

  annotations {
    mode = "write"
  }

  param "session_id" {
    type        = "string"
    required    = true
    description = "Browser session ID"
  }

  param "url" {
    type        = "string"
    required    = true
    description = "URL to navigate to"
  }

  operation {
    protocol = "browser"
    browser {
      session_id = "{{ args.session_id }}"
      steps = [
        { action = "navigate", url = "{{ args.url }}" },
        { action = "snapshot" },
      ]
    }
  }

  result {
    decode = "json"
    output = "Navigated to {{ args.url }}. Page state:\n{{ result.text | truncate(500) }}"
  }
}
