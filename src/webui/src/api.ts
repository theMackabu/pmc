import ky from 'ky';
import { $settings } from '@/store';

export { SSE } from 'sse.js';

export const headers = { token: $settings.get().token };

export const api = ky.create({ headers });
