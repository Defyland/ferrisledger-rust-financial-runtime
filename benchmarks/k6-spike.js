import smoke from './k6-smoke.js';

export const options = {
  stages: [
    { duration: '10s', target: 5 },
    { duration: '10s', target: 50 },
    { duration: '20s', target: 50 },
    { duration: '10s', target: 0 },
  ],
  thresholds: {
    http_req_failed: ['rate<0.10'],
    http_req_duration: ['p(99)<2500'],
  },
};

export default smoke;
