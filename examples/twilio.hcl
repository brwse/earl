version = 1
provider = "twilio"
categories = ["messaging", "voice", "telephony"]

command "send_message" {
  title       = "Send SMS/MMS"
  summary     = "Send a text message or MMS to a phone number"
  description = "Send an SMS or MMS message via Twilio. Requires To, From (E.164 format), and Body. Optionally attach media for MMS."
  categories  = ["messaging"]

  annotations {
    mode    = "write"
    secrets = ["twilio.account_sid", "twilio.auth_token"]
  }

  param "to" {
    type        = "string"
    required    = true
    description = "Recipient phone number in E.164 format (e.g. +14155551234)"
  }

  param "from" {
    type        = "string"
    required    = true
    description = "Sender phone number in E.164 format (must be a Twilio number)"
  }

  param "body" {
    type        = "string"
    required    = true
    description = "Message text (max 1600 characters)"
  }

  param "media_url" {
    type        = "string"
    required    = false
    description = "URL of media to attach (sends as MMS)"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://api.twilio.com/2010-04-01/Accounts/{{ secrets.twilio_account_sid }}/Messages.json"

    auth {
      kind            = "basic"
      username        = "{{ secrets.twilio_account_sid }}"
      password_secret = "twilio.auth_token"
    }

    body {
      kind = "form_urlencoded"
      fields = {
        To       = "{{ args.to }}"
        From     = "{{ args.from }}"
        Body     = "{{ args.body }}"
        MediaUrl = "{{ args.media_url }}"
      }
    }
  }

  result {
    decode = "json"
    output = "Message {{ result.sid }} {{ result.status }}\nFrom: {{ result.from }} → To: {{ result.to }}\nBody: {{ result.body }}\nSegments: {{ result.num_segments }} | Price: {{ result.price }} {{ result.price_unit }}"
  }
}

command "list_messages" {
  title       = "List messages"
  summary     = "List recent SMS/MMS messages"
  description = "Retrieve a list of messages sent from or received by your Twilio account. Supports filtering by sender, recipient, and date."
  categories  = ["messaging"]

  annotations {
    mode    = "read"
    secrets = ["twilio.account_sid", "twilio.auth_token"]
  }

  param "to" {
    type        = "string"
    required    = false
    description = "Filter by recipient phone number"
  }

  param "from" {
    type        = "string"
    required    = false
    description = "Filter by sender phone number"
  }

  param "page_size" {
    type        = "integer"
    required    = false
    default     = 20
    description = "Number of results per page (1-1000)"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.twilio.com/2010-04-01/Accounts/{{ secrets.twilio_account_sid }}/Messages.json"

    auth {
      kind            = "basic"
      username        = "{{ secrets.twilio_account_sid }}"
      password_secret = "twilio.auth_token"
    }

    query = {
      To       = "{{ args.to }}"
      From     = "{{ args.from }}"
      PageSize = "{{ args.page_size }}"
    }
  }

  result {
    decode = "json"
    extract { json_pointer = "/messages" }
    output = "{{ result | length }} messages returned"
  }
}

command "get_message" {
  title       = "Get message"
  summary     = "Get details of a specific message by SID"
  description = "Retrieve full details of a single SMS/MMS message by its SID."
  categories  = ["messaging"]

  annotations {
    mode    = "read"
    secrets = ["twilio.account_sid", "twilio.auth_token"]
  }

  param "message_sid" {
    type        = "string"
    required    = true
    description = "Message SID (starts with SM or MM)"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.twilio.com/2010-04-01/Accounts/{{ secrets.twilio_account_sid }}/Messages/{{ args.message_sid }}.json"

    auth {
      kind            = "basic"
      username        = "{{ secrets.twilio_account_sid }}"
      password_secret = "twilio.auth_token"
    }
  }

  result {
    decode = "json"
    output = "Message {{ result.sid }} ({{ result.status }})\nDirection: {{ result.direction }}\nFrom: {{ result.from }} → To: {{ result.to }}\nBody: {{ result.body }}\nSent: {{ result.date_sent }} | Segments: {{ result.num_segments }} | Price: {{ result.price }} {{ result.price_unit }}"
  }
}

command "delete_message" {
  title       = "Delete message"
  summary     = "Delete a message by SID"
  description = "Permanently delete an SMS/MMS message from your account by its SID."
  categories  = ["messaging"]

  annotations {
    mode    = "write"
    secrets = ["twilio.account_sid", "twilio.auth_token"]
  }

  param "message_sid" {
    type        = "string"
    required    = true
    description = "Message SID to delete (starts with SM or MM)"
  }

  operation {
    protocol = "http"
    method   = "DELETE"
    url      = "https://api.twilio.com/2010-04-01/Accounts/{{ secrets.twilio_account_sid }}/Messages/{{ args.message_sid }}.json"

    auth {
      kind            = "basic"
      username        = "{{ secrets.twilio_account_sid }}"
      password_secret = "twilio.auth_token"
    }
  }

  result {
    decode = "json"
    output = "Deleted message {{ args.message_sid }}"
  }
}

command "create_call" {
  title       = "Create call"
  summary     = "Initiate an outbound phone call"
  description = "Place an outbound call via Twilio. Provide TwiML instructions inline or via a URL to control call behavior."
  categories  = ["voice"]

  annotations {
    mode    = "write"
    secrets = ["twilio.account_sid", "twilio.auth_token"]
  }

  param "to" {
    type        = "string"
    required    = true
    description = "Recipient phone number in E.164 format"
  }

  param "from" {
    type        = "string"
    required    = true
    description = "Caller phone number (must be a Twilio number or verified caller ID)"
  }

  param "twiml" {
    type        = "string"
    required    = false
    description = "Inline TwiML instructions (e.g. '<Response><Say>Hello</Say></Response>')"
  }

  param "url" {
    type        = "string"
    required    = false
    description = "URL returning TwiML (alternative to twiml param)"
  }

  param "timeout" {
    type        = "integer"
    required    = false
    default     = 60
    description = "Seconds to wait for the call to be answered"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://api.twilio.com/2010-04-01/Accounts/{{ secrets.twilio_account_sid }}/Calls.json"

    auth {
      kind            = "basic"
      username        = "{{ secrets.twilio_account_sid }}"
      password_secret = "twilio.auth_token"
    }

    body {
      kind = "form_urlencoded"
      fields = {
        To      = "{{ args.to }}"
        From    = "{{ args.from }}"
        Twiml   = "{{ args.twiml }}"
        Url     = "{{ args.url }}"
        Timeout = "{{ args.timeout }}"
      }
    }
  }

  result {
    decode = "json"
    output = "Call {{ result.sid }} {{ result.status }}\nFrom: {{ result.from_formatted }} → To: {{ result.to_formatted }}\nDirection: {{ result.direction }}"
  }
}

command "list_calls" {
  title       = "List calls"
  summary     = "List recent phone calls"
  description = "Retrieve a list of calls made to or from your Twilio account. Supports filtering by phone number and status."
  categories  = ["voice"]

  annotations {
    mode    = "read"
    secrets = ["twilio.account_sid", "twilio.auth_token"]
  }

  param "to" {
    type        = "string"
    required    = false
    description = "Filter by recipient phone number"
  }

  param "from" {
    type        = "string"
    required    = false
    description = "Filter by caller phone number"
  }

  param "status" {
    type        = "string"
    required    = false
    description = "Filter by status: queued, ringing, in-progress, completed, busy, failed, no-answer, canceled"
  }

  param "page_size" {
    type        = "integer"
    required    = false
    default     = 20
    description = "Number of results per page (1-1000)"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.twilio.com/2010-04-01/Accounts/{{ secrets.twilio_account_sid }}/Calls.json"

    auth {
      kind            = "basic"
      username        = "{{ secrets.twilio_account_sid }}"
      password_secret = "twilio.auth_token"
    }

    query = {
      To       = "{{ args.to }}"
      From     = "{{ args.from }}"
      Status   = "{{ args.status }}"
      PageSize = "{{ args.page_size }}"
    }
  }

  result {
    decode = "json"
    extract { json_pointer = "/calls" }
    output = "{{ result | length }} calls returned"
  }
}

command "get_call" {
  title       = "Get call"
  summary     = "Get details of a specific call by SID"
  description = "Retrieve full details of a phone call by its SID, including duration, price, and status."
  categories  = ["voice"]

  annotations {
    mode    = "read"
    secrets = ["twilio.account_sid", "twilio.auth_token"]
  }

  param "call_sid" {
    type        = "string"
    required    = true
    description = "Call SID (starts with CA)"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.twilio.com/2010-04-01/Accounts/{{ secrets.twilio_account_sid }}/Calls/{{ args.call_sid }}.json"

    auth {
      kind            = "basic"
      username        = "{{ secrets.twilio_account_sid }}"
      password_secret = "twilio.auth_token"
    }
  }

  result {
    decode = "json"
    output = "Call {{ result.sid }} ({{ result.status }})\nFrom: {{ result.from_formatted }} → To: {{ result.to_formatted }}\nDirection: {{ result.direction }} | Start: {{ result.start_time }} | End: {{ result.end_time }}\nDuration: {{ result.duration }}s | Price: {{ result.price }} {{ result.price_unit }}"
  }
}

command "update_call" {
  title       = "Update call"
  summary     = "Modify or end a live call"
  description = "Update a live call by redirecting it to new TwiML, or end it by setting status to completed or canceled."
  categories  = ["voice"]

  annotations {
    mode    = "write"
    secrets = ["twilio.account_sid", "twilio.auth_token"]
  }

  param "call_sid" {
    type        = "string"
    required    = true
    description = "Call SID to update (starts with CA)"
  }

  param "status" {
    type        = "string"
    required    = false
    description = "Set to 'completed' to hang up or 'canceled' to cancel a queued call"
  }

  param "twiml" {
    type        = "string"
    required    = false
    description = "New TwiML instructions to redirect the live call"
  }

  param "url" {
    type        = "string"
    required    = false
    description = "New TwiML URL to redirect the live call"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://api.twilio.com/2010-04-01/Accounts/{{ secrets.twilio_account_sid }}/Calls/{{ args.call_sid }}.json"

    auth {
      kind            = "basic"
      username        = "{{ secrets.twilio_account_sid }}"
      password_secret = "twilio.auth_token"
    }

    body {
      kind = "form_urlencoded"
      fields = {
        Status = "{{ args.status }}"
        Twiml  = "{{ args.twiml }}"
        Url    = "{{ args.url }}"
      }
    }
  }

  result {
    decode = "json"
    output = "Call {{ result.sid }} updated → {{ result.status }}"
  }
}

command "list_incoming_numbers" {
  title       = "List phone numbers"
  summary     = "List your Twilio phone numbers"
  description = "List all phone numbers owned by your Twilio account, with their capabilities and configuration."
  categories  = ["telephony"]

  annotations {
    mode    = "read"
    secrets = ["twilio.account_sid", "twilio.auth_token"]
  }

  param "phone_number" {
    type        = "string"
    required    = false
    description = "Filter by exact phone number (E.164)"
  }

  param "friendly_name" {
    type        = "string"
    required    = false
    description = "Filter by friendly name"
  }

  param "page_size" {
    type        = "integer"
    required    = false
    default     = 20
    description = "Number of results per page (1-1000)"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.twilio.com/2010-04-01/Accounts/{{ secrets.twilio_account_sid }}/IncomingPhoneNumbers.json"

    auth {
      kind            = "basic"
      username        = "{{ secrets.twilio_account_sid }}"
      password_secret = "twilio.auth_token"
    }

    query = {
      PhoneNumber  = "{{ args.phone_number }}"
      FriendlyName = "{{ args.friendly_name }}"
      PageSize     = "{{ args.page_size }}"
    }
  }

  result {
    decode = "json"
    extract { json_pointer = "/incoming_phone_numbers" }
    output = "{{ result | length }} phone numbers"
  }
}

command "list_available_numbers" {
  title       = "Search available numbers"
  summary     = "Search for phone numbers available for purchase"
  description = "Search Twilio's inventory of available local phone numbers by country, area code, or capabilities."
  categories  = ["telephony"]

  annotations {
    mode    = "read"
    secrets = ["twilio.account_sid", "twilio.auth_token"]
  }

  param "country_code" {
    type        = "string"
    required    = true
    description = "ISO 3166-1 alpha-2 country code (e.g. US, GB, CA)"
  }

  param "area_code" {
    type        = "integer"
    required    = false
    description = "Filter by area code (US/Canada)"
  }

  param "contains" {
    type        = "string"
    required    = false
    description = "Pattern match (* = any digit, e.g. *867*)"
  }

  param "in_region" {
    type        = "string"
    required    = false
    description = "State or province abbreviation (e.g. CA, NY)"
  }

  param "sms_enabled" {
    type        = "boolean"
    required    = false
    description = "Only return SMS-capable numbers"
  }

  param "page_size" {
    type        = "integer"
    required    = false
    default     = 20
    description = "Number of results per page"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.twilio.com/2010-04-01/Accounts/{{ secrets.twilio_account_sid }}/AvailablePhoneNumbers/{{ args.country_code }}/Local.json"

    auth {
      kind            = "basic"
      username        = "{{ secrets.twilio_account_sid }}"
      password_secret = "twilio.auth_token"
    }

    query = {
      AreaCode   = "{{ args.area_code }}"
      Contains   = "{{ args.contains }}"
      InRegion   = "{{ args.in_region }}"
      SmsEnabled = "{{ args.sms_enabled }}"
      PageSize   = "{{ args.page_size }}"
    }
  }

  result {
    decode = "json"
    extract { json_pointer = "/available_phone_numbers" }
    output = "{{ result | length }} numbers available"
  }
}

command "purchase_number" {
  title       = "Purchase number"
  summary     = "Buy a phone number from Twilio's inventory"
  description = "Purchase an available phone number and add it to your Twilio account."
  categories  = ["telephony"]

  annotations {
    mode    = "write"
    secrets = ["twilio.account_sid", "twilio.auth_token"]
  }

  param "phone_number" {
    type        = "string"
    required    = true
    description = "Phone number to purchase in E.164 format"
  }

  param "friendly_name" {
    type        = "string"
    required    = false
    description = "Human-readable label for this number (max 64 chars)"
  }

  param "voice_url" {
    type        = "string"
    required    = false
    description = "Webhook URL for incoming voice calls"
  }

  param "sms_url" {
    type        = "string"
    required    = false
    description = "Webhook URL for incoming SMS"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://api.twilio.com/2010-04-01/Accounts/{{ secrets.twilio_account_sid }}/IncomingPhoneNumbers.json"

    auth {
      kind            = "basic"
      username        = "{{ secrets.twilio_account_sid }}"
      password_secret = "twilio.auth_token"
    }

    body {
      kind = "form_urlencoded"
      fields = {
        PhoneNumber  = "{{ args.phone_number }}"
        FriendlyName = "{{ args.friendly_name }}"
        VoiceUrl     = "{{ args.voice_url }}"
        SmsUrl       = "{{ args.sms_url }}"
      }
    }
  }

  result {
    decode = "json"
    output = "Purchased {{ result.phone_number }} ({{ result.friendly_name }})\nSID: {{ result.sid }}\nVoice: {{ result.capabilities.voice }} | SMS: {{ result.capabilities.sms }} | MMS: {{ result.capabilities.mms }}"
  }
}

command "release_number" {
  title       = "Release number"
  summary     = "Release a phone number from your account"
  description = "Remove a phone number from your Twilio account. This cannot be undone."
  categories  = ["telephony"]

  annotations {
    mode    = "write"
    secrets = ["twilio.account_sid", "twilio.auth_token"]
  }

  param "phone_sid" {
    type        = "string"
    required    = true
    description = "SID of the phone number to release (starts with PN)"
  }

  operation {
    protocol = "http"
    method   = "DELETE"
    url      = "https://api.twilio.com/2010-04-01/Accounts/{{ secrets.twilio_account_sid }}/IncomingPhoneNumbers/{{ args.phone_sid }}.json"

    auth {
      kind            = "basic"
      username        = "{{ secrets.twilio_account_sid }}"
      password_secret = "twilio.auth_token"
    }
  }

  result {
    decode = "json"
    output = "Released phone number {{ args.phone_sid }}"
  }
}

command "lookup_phone_number" {
  title       = "Lookup phone number"
  summary     = "Look up information about a phone number"
  description = "Query the Twilio Lookup API for carrier, caller name, and validation data on any phone number."
  categories  = ["telephony"]

  annotations {
    mode    = "read"
    secrets = ["twilio.account_sid", "twilio.auth_token"]
  }

  param "phone_number" {
    type        = "string"
    required    = true
    description = "Phone number to look up in E.164 format"
  }

  param "fields" {
    type        = "string"
    required    = false
    description = "Comma-separated data fields: validation, line_type_intelligence, caller_name, sim_swap, sms_pumping_risk"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://lookups.twilio.com/v2/PhoneNumbers/{{ args.phone_number }}"

    auth {
      kind            = "basic"
      username        = "{{ secrets.twilio_account_sid }}"
      password_secret = "twilio.auth_token"
    }

    query = {
      Fields = "{{ args.fields }}"
    }
  }

  result {
    decode = "json"
    output = "{{ result.phone_number }} ({{ result.national_format }})\nCountry: {{ result.country_code }} | Valid: {{ result.valid }}"
  }
}

command "get_account" {
  title       = "Get account info"
  summary     = "Get your Twilio account details"
  description = "Retrieve details about your Twilio account including name, status, and type."
  categories  = ["telephony"]

  annotations {
    mode    = "read"
    secrets = ["twilio.account_sid", "twilio.auth_token"]
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.twilio.com/2010-04-01/Accounts/{{ secrets.twilio_account_sid }}.json"

    auth {
      kind            = "basic"
      username        = "{{ secrets.twilio_account_sid }}"
      password_secret = "twilio.auth_token"
    }
  }

  result {
    decode = "json"
    output = "Account {{ result.sid }}\nName: {{ result.friendly_name }}\nStatus: {{ result.status }} | Type: {{ result.type }}\nCreated: {{ result.date_created }}"
  }
}
