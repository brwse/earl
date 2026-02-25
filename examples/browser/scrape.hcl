version  = 1
provider = "browser"
categories = ["browser", "scraping"]

command "get_text" {
  title       = "Get page text"
  summary     = "Extract visible text from a JavaScript-rendered page"
  description = "Navigate to a URL and return document.body.innerText."

  annotations {
    mode = "read"
  }

  param "url" {
    type        = "string"
    required    = true
    description = "URL to scrape"
  }

  operation {
    protocol = "browser"
    browser {
      headless = true
      steps = [
        { action = "navigate", url = "{{ args.url }}" },
        { action = "evaluate", function = "() => document.body.innerText" },
      ]
    }
  }

  result {
    decode = "json"
    output = "{{ result.value }}"
  }
}
