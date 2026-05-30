import smoke from './k6-smoke.js';

export const options = {
  vus: 10,
  duration: '1m',
  thresholds: {
    http_req_failed: ['rate<0.01'],
    http_req_duration: ['p(95)<750', 'p(99)<1200'],
  },
};

export default smoke;
