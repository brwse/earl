version  = 1
provider = "browser"
categories = ["browser", "capture"]

command "screenshot" {
  title       = "Screenshot"
  summary     = "Take a screenshot of a URL"
  description = "Navigate to a URL and capture a full-page screenshot."

  annotations {
    mode = "read"
  }

  param "url" {
    type        = "string"
    required    = true
    description = "URL to screenshot"
  }

  operation {
    protocol = "browser"
    browser {
      headless = true
      steps = [
        { action = "navigate",   url = "{{ args.url }}" },
        { action = "screenshot", full_page = true },
      ]
    }
  }

  result {
    decode = "json"
    output = "Screenshot saved to {{ result.path }}"
  }
}
