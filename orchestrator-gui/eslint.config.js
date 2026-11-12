import js from '@eslint/js';
import tseslint from 'typescript-eslint';
import reactHooks from 'eslint-plugin-react-hooks';

/**
 * ESLint flat configuration.
 *
 * ESLint 9 uses `eslint.config.js`; the old `.eslintrc` format is no longer read.
 */
export default tseslint.config(
  {
    ignores: ['dist/**', 'node_modules/**', 'coverage/**', 'playwright-report/**'],
  },
  js.configs.recommended,
  ...tseslint.configs.recommended,
  {
    files: ['**/*.{ts,tsx}'],
    plugins: {
      'react-hooks': reactHooks,
    },
    rules: {
      ...reactHooks.configs.recommended.rules,
      // TypeScript already reports unused variables, and it understands type-only usage.
      '@typescript-eslint/no-unused-vars': [
        'error',
        { argsIgnorePattern: '^_', varsIgnorePattern: '^_' },
      ],
    },
  },
  {
    files: ['vite.config.ts', 'vitest.config.ts'],
    languageOptions: {
      globals: {
        process: 'readonly',
      },
    },
  },
);
