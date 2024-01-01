import ky from 'ky';
import { $settings } from '@/store';

export const api = ky.create({
	headers: { Authorization: 'token ' + $settings.get().token },
});
