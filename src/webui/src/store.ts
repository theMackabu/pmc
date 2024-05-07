import { persistentMap } from '@nanostores/persistent';

export interface SettingsStore {
	token?: string;
}

export const $settings = persistentMap<SettingsStore>('settings:', {});
