/* eslint-env node */
module.exports = {
  root: true,
  env: { browser: true, es2022: true },
  extends: ['eslint:recommended', 'plugin:react-hooks/recommended'],
  parserOptions: { ecmaVersion: 'latest', sourceType: 'module' },
  ignorePatterns: ['dist', 'node_modules', 'coverage'],
};
