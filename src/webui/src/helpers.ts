import { useState } from 'react';

export const classNames = (...classes: Array<any>) => classes.filter(Boolean).join(' ');

export const isRunning = (status: string): boolean => (status == 'stopped' ? false : status == 'crashed' ? false : true);

export const formatMemory = (bytes: number): [number, string] => {
	const units = ['b', 'kb', 'mb', 'gb'];
	let size = bytes;
	let unitIndex = 0;

	while (size > 1024 && unitIndex < units.length - 1) {
		size /= 1024;
		unitIndex++;
	}

	return [+size.toFixed(1), units[unitIndex]];
};

export const startDuration = (input: string, split: boolean = true): [number, string] | string => {
	const match = input.match(/(\d+)([dhms])/);
	if (!match) return null;

	const [number, unit] = [parseInt(match[1]), 'dhms'.indexOf(match[2])];
	const fullUnit = ['day', 'hour', 'minute', 'second'][unit] + (number !== 1 ? 's' : '');

	return split ? [number, fullUnit] : `${number} ${fullUnit}`;
};

export const isVersionTooFar = (currentVersion: string, newVersion: string): boolean => {
	const parseVersion = (version) => version.split('.').map(Number);

	const [currentMajor, currentMinor, currentPatch] = parseVersion(currentVersion);
	const [newMajor, newMinor, newPatch] = parseVersion(newVersion);

	if (newMajor > currentMajor + 1) {
		return true;
	} else if (newMajor === currentMajor + 1 && newMinor > 0) {
		return true;
	} else if (newMajor === currentMajor && newMinor > currentMinor + 2) {
		return true;
	}

	return false;
};

export const useArray = (initialValue = [], maxSize = 5) => {
	const [value, setValue] = useState(initialValue);

	const clear = () => setValue([]);
	const count = () => value.length;
	const isEmpty = () => value.length === 0;
	const push = (element) => setValue((oldValue) => [...oldValue, element]);
	const remove = (index) => setValue((oldValue) => oldValue.filter((_, i) => i !== index));

	const pushMax = (element) =>
		setValue((oldValue) => {
			const newValue = [...oldValue, element];
			if (newValue.length > maxSize) {
				newValue.shift();
			}
			return newValue;
		});

	return { value, setValue, clear, count, isEmpty, push, remove, pushMax };
};
