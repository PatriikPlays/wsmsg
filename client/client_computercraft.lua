local expect = require("cc.expect")

function split(str, sep)
  sep = sep or "%s"
  local ret, index = {}, 1
  for match in string.gmatch(str, "([^"..sep.."]+)") do
    ret[index] = match
    index = index + 1
  end
  return ret
end

local lib = {}

lib.new = function(url, tokens)
  expect(1, url, "string")
  expect(2, tokens, "table", "nil")

  url = url .. "/"
  for i=1,32 do
    -- the server doesnt use this, its required for multiple connections (the ws events only expose url)
    url = url .. string.format("%x", math.random(0,15))
  end

  local ws, err = http.websocket(url, {
    ["X-Token"] = table.concat(tokens or {}, ",")
  })

  if not ws then error("Failed to connect to ws: "..(err or "Unknown error")) end

  local t = { internal = { messageId = 1; url = url, tokens = tokens, ws = ws } }

  t.enum = {
    MessageTypeC2S = {
      Ping = 0,
      ListSubscribed = 1,
      Subscribe = 2,
      Unsubscribe = 3,
      Message = 255
    },
    MessageTypeS2C = {
      Reply = 0,
      Message = 255
    }
  }

  t.sendRaw = function(self, type, data)
    expect(1, self, "table")
    expect(2, type, "number") -- FIXME: check if its a byte
    expect(3, data, "string", "nil")

    local id = string.pack(">I8",  self.internal.messageId)
    self.internal.ws.send(string.char(type)..id..(data or ""), true)
    self.internal.messageId = self.internal.messageId + 1
  end

  t.sendAndReceive = function(self, type, data) -- FIXME: add timeout
    expect(1, self, "table")
    expect(2, type, "number") -- FIXME: check if its a byte
    expect(3, data, "string", "nil")

    local id = string.pack(">I8",  self.internal.messageId)
    self.internal.ws.send(string.char(type)..id..(data or ""), true)
    self.internal.messageId = self.internal.messageId + 1

    while true do
      local data = self.internal.ws.receive()
      if string.byte(data:sub(1,1)) == self.enum.MessageTypeS2C.Reply and data:sub(2,9) == id then
        local code = string.byte(data:sub(10,10))
        data = data:sub(11)

        return code, data
      end
    end
  end

  t.ping = function(self, payload)
    expect(1, self, "table")
    expect(2, payload, "string")
    return self:sendAndReceive(self.enum.MessageTypeC2S.Ping, payload)
  end

  t.subscribe = function(self, channel)
    expect(1, self, "table")
    expect(2, channel, "string")
    return self:sendAndReceive(self.enum.MessageTypeC2S.Subscribe, channel)
  end

  t.unsubscribe = function(self, channel)
    expect(1, self, "table")
    expect(2, channel, "string")
    return self:sendAndReceive(self.enum.MessageTypeC2S.Unsubscribe, channel)
  end

  t.listSubscribed = function(self)
    expect(1, self, "table")
    return self:sendAndReceive(self.enum.MessageTypeC2S.ListSubscribed)
  end

  t.receive = function(self)
    expect(1, self, "table")
    while true do
      local d = self.internal.ws.receive()
      if d then
        if string.byte(d:sub(1,1)) == self.enum.MessageTypeS2C.Message then
          return d:sub(2,33), d:sub(34)
        end
      end
    end
  end

  t.send = function(self, target, data)
    expect(1, self, "table")
    expect(2, target, "string")
    expect(3, data, "string")

    self.internal.ws.send(string.char(self.enum.MessageTypeC2S.Message).."\000\000\000\000\000\000\000\000"..target..data, true)
  end

  t.connectionMaintainer = function(self)
    while true do
      print("pinging")
      self:ping("keepalive")
      print("ok")
      sleep(10)
    end
  end

  return t
end

return lib
