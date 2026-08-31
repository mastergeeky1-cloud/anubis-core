/**
 * ANUBIS WS — lightweight opcode client for the real-time transport.
 *
 * Usage:
 *   const anubis = new AnubisWS('ws://localhost:7600/ws');
 *   await anubis.connect({ lang: 'en', voice: 'en_US-amy-medium' });
 *
 *   anubis.on('text', (text) => { /* live LLM delta */
 *   });
 *   anubis.on('voice', (opusBytes) => { /* audio chunk */
 *   });
 *   anubis.on('status', (status) => { /* thinking / synthesizing / done */
 *   });
 *   anubis.on('meta', (data) => { /* voice, lang, credits */
 *   });
 *   anubis.on('error', (err) => { /* { code, message } */
 *   });
 *   anubis.on('history', (turns) => { /* [{role,content}] */
 *   });
 *   anubis.on('hello', (data) => { /* protocol version, session id */
 *   });
 *
 *   await anubis.ask('What is the meaning of life?');
 *   anubis.setVoice('am_adam');
 *   anubis.setLang('ar');
 *   await anubis.requestHistory();
 *   anubis.disconnect();
 */

// ── Opcodes (mirror src/ws/codec.rs) ─────────────────────────────────────────

const CLIENT = {
  Hello:   0x01,
  Text:    0x02,
  Voice:   0x03,
  Config:  0x04,
  Ping:    0x05,
  History: 0x06,
};

const SERVER = {
  Hello:     0x81,
  TextDelta: 0x82,
  VoiceChunk:0x83,
  Status:    0x84,
  Error:     0x85,
  Meta:      0x86,
  Pong:      0x87,
  History:   0x88,
  TextEnd:   0x89,
};

/**
 * Encode a binary frame: [1B opcode][4B length][payload]
 */
function encodeFrame(opcode, payload) {
  const data = typeof payload === 'string'
    ? new TextEncoder().encode(payload)
    : payload;
  const buf = new ArrayBuffer(5 + data.length);
  const view = new DataView(buf);
  view.setUint8(0, opcode);
  view.setUint32(1, data.length, false); // big-endian
  new Uint8Array(buf, 5).set(data instanceof Uint8Array ? data : new Uint8Array(data));
  return buf;
}

/**
 * Decode incoming binary data. Handles partial frames and concatenation.
 */
function decodeFrames(raw) {
  const frames = [];
  const bytes = new Uint8Array(raw);
  let offset = 0;

  while (offset < bytes.length) {
    if (offset + 5 > bytes.length) break; // incomplete header
    const opcode = bytes[offset];
    const len = (bytes[offset+1] << 24)
              | (bytes[offset+2] << 16)
              | (bytes[offset+3] <<  8)
              | bytes[offset+4];
    if (len > 4 * 1024 * 1024) break; // malformed — skip
    if (offset + 5 + len > bytes.length) break; // incomplete payload
    const payload = bytes.slice(offset + 5, offset + 5 + len);
    frames.push({ opcode, payload });
    offset += 5 + len;
  }
  return frames;
}

// ── Client ───────────────────────────────────────────────────────────────────

export class AnubisWS extends EventTarget {
  constructor(url) {
    super();
    this.url = url;
    this.ws = null;
    this.sessionId = null;
    this._connected = false;
    this._buffer = new Uint8Array(0);
  }

  /**
   * Open the connection and complete the Hello handshake.
   * @param {Object} opts — { token?, lang?, voice? }
   * @returns {Promise<Object>} — server Hello payload
   */
  async connect(opts = {}) {
    return new Promise((resolve, reject) => {
      this.ws = new WebSocket(this.url);
      this.ws.binaryType = 'arraybuffer';

      this.ws.onopen = () => {
        const helloPayload = JSON.stringify({
          token: opts.token || undefined,
          lang: opts.lang || 'en',
          voice: opts.voice || 'en_US-amy-medium',
        });
        this.ws.send(encodeFrame(CLIENT.Hello, helloPayload));
      };

      this.ws.onmessage = (event) => {
        const raw = new Uint8Array(event.data);
        const merged = new Uint8Array(this._buffer.length + raw.length);
        merged.set(this._buffer);
        merged.set(raw, this._buffer.length);
        const frames = decodeFrames(merged);

        // Keep leftover bytes.
        let consumed = 0;
        for (const f of frames) consumed += 5 + f.payload.length;
        this._buffer = merged.slice(consumed);

        for (const { opcode, payload } of frames) {
          const text = new TextDecoder().decode(payload);
          let data;
          try { data = JSON.parse(text); } catch { data = text; }

          switch (opcode) {
            case SERVER.Hello:
              this.sessionId = data.session_id;
              this._connected = true;
              this.dispatchEvent(new CustomEvent('hello', { detail: data }));
              resolve(data);
              break;
            case SERVER.TextDelta:
              this.dispatchEvent(new CustomEvent('text', { detail: data }));
              break;
            case SERVER.VoiceChunk:
              // payload is raw opus/ogg bytes — pass through as ArrayBuffer.
              this.dispatchEvent(new CustomEvent('voice', { detail: payload }));
              break;
            case SERVER.Status:
              this.dispatchEvent(new CustomEvent('status', { detail: data }));
              break;
            case SERVER.Error:
              this.dispatchEvent(new CustomEvent('error', { detail: data }));
              break;
            case SERVER.Meta:
              this.dispatchEvent(new CustomEvent('meta', { detail: data }));
              break;
            case SERVER.Pong:
              this.dispatchEvent(new CustomEvent('pong', { detail: data }));
              break;
            case SERVER.History:
              this.dispatchEvent(new CustomEvent('history', { detail: data }));
              break;
            case SERVER.TextEnd:
              this.dispatchEvent(new CustomEvent('textend', { detail: data }));
              break;
          }
        }
      };

      this.ws.onerror = () => reject(new Error('WebSocket connection failed'));
      this.ws.onclose = () => {
        this._connected = false;
        this.dispatchEvent(new Event('disconnect'));
      };
    });
  }

  /** Send a text prompt to the brain and receive streaming deltas. */
  async ask(text) {
    this._ensureConnected();
    this.ws.send(encodeFrame(CLIENT.Text, text));
  }

  /** Send a raw audio chunk to be transcribed. */
  async sendVoice(audioBuffer) {
    this._ensureConnected();
    this.ws.send(encodeFrame(CLIENT.Voice,
      audioBuffer instanceof ArrayBuffer ? new Uint8Array(audioBuffer) : audioBuffer
    ));
  }

  /** Update runtime voice/language settings. */
  setVoice(voiceId) {
    this._ensureConnected();
    this.ws.send(encodeFrame(CLIENT.Config, JSON.stringify({ voice: voiceId })));
  }

  setLang(lang) {
    this._ensureConnected();
    this.ws.send(encodeFrame(CLIENT.Config, JSON.stringify({ lang })));
  }

  /** Send a heartbeat ping. */
  ping() {
    this._ensureConnected();
    this.ws.send(encodeFrame(CLIENT.Ping, '{}'));
  }

  /** Request the conversation history for the session. */
  requestHistory() {
    this._ensureConnected();
    this.ws.send(encodeFrame(CLIENT.History, '{}'));
  }

  /** Gracefully close the connection. */
  disconnect() {
    if (this.ws) this.ws.close();
  }

  get connected() {
    return this._connected;
  }

  _ensureConnected() {
    if (!this._connected || !this.ws) {
      throw new Error('Not connected — call connect() first');
    }
  }
}

export default AnubisWS;
