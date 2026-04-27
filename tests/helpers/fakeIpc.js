function clone(value) {
  if (value === undefined) return undefined;
  return JSON.parse(JSON.stringify(value));
}

function createFakeIpcRenderer(responses = {}) {
  const ipc = {
    invokes: [],
    sends: [],
    listeners: new Map(),

    async invoke(channel, ...args) {
      this.invokes.push({ channel, args: clone(args) });
      const response = responses[channel];

      if (typeof response === 'function') {
        return response(...args);
      }

      return clone(response);
    },

    send(channel, ...args) {
      this.sends.push({ channel, args: clone(args) });
    },

    on(channel, listener) {
      this.listeners.set(channel, listener);
    },

    emit(channel, ...args) {
      const listener = this.listeners.get(channel);
      if (listener) {
        listener({ sender: this }, ...args);
      }
    },

    calls(channel) {
      return this.invokes.filter(call => call.channel === channel);
    },

    sent(channel) {
      return this.sends.filter(call => call.channel === channel);
    }
  };

  return ipc;
}

module.exports = {
  createFakeIpcRenderer
};
