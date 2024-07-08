import { useState, useCallback } from 'react';

export const classNames = (...classes: Array<any>) => classes.filter(Boolean).join(' ');
