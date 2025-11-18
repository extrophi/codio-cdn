/**
 * Jest test setup for browser tests
 * Configures test environment and global mocks
 */

// Increase timeout for integration tests
jest.setTimeout(30000);

// Mock browser APIs that may not be available in test environment
global.RTCPeerConnection = global.RTCPeerConnection || class RTCPeerConnection {
  constructor() {
    this.localDescription = null;
    this.remoteDescription = null;
    this.connectionState = 'new';
  }

  async createOffer() {
    return { type: 'offer', sdp: 'mock-sdp' };
  }

  async createAnswer() {
    return { type: 'answer', sdp: 'mock-sdp' };
  }

  async setLocalDescription(desc) {
    this.localDescription = desc;
  }

  async setRemoteDescription(desc) {
    this.remoteDescription = desc;
  }

  createDataChannel(label) {
    return {
      label,
      readyState: 'open',
      send: jest.fn(),
      close: jest.fn(),
    };
  }

  close() {
    this.connectionState = 'closed';
  }
};

global.RTCDataChannel = global.RTCDataChannel || class RTCDataChannel {
  constructor(label) {
    this.label = label;
    this.readyState = 'connecting';
  }

  send(data) {
    // Mock implementation
  }

  close() {
    this.readyState = 'closed';
  }
};

// Mock Service Worker API
if (!global.navigator.serviceWorker) {
  global.navigator.serviceWorker = {
    register: jest.fn().mockResolvedValue({
      active: {},
      installing: null,
      waiting: null,
      scope: '/',
      unregister: jest.fn().mockResolvedValue(true),
    }),
    ready: Promise.resolve({
      active: {},
      scope: '/',
    }),
    getRegistration: jest.fn().mockResolvedValue({
      active: {},
      scope: '/',
      unregister: jest.fn().mockResolvedValue(true),
    }),
  };
}

// Mock fetch for testing
global.fetch = global.fetch || jest.fn();

// Mock crypto for CID generation
if (!global.crypto) {
  global.crypto = {
    subtle: {
      digest: jest.fn().mockImplementation(async (algorithm, data) => {
        // Simple mock - in real tests this would use actual SHA-256
        return new ArrayBuffer(32);
      }),
    },
  };
}

// Console helpers for debugging
global.logTestInfo = (message) => {
  if (process.env.DEBUG_TESTS) {
    console.log(`[TEST INFO] ${message}`);
  }
};

// Cleanup after each test
afterEach(() => {
  jest.clearAllMocks();
});
