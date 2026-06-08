import { languages as codeMirrorLanguages } from '@codemirror/language-data';

export type LanguageOption = {
  value: string;
  label: string;
  extension: string;
};

export const plainTextLanguage = {
  value: 'Plain Text',
  label: 'Plain Text',
  extension: 'txt',
};

export const languageOptions: LanguageOption[] = [
  plainTextLanguage,
  ...codeMirrorLanguages
    .map((language) => ({
      value: language.name,
      label: language.name,
      extension: language.extensions[0] ?? 'txt',
    }))
    .sort((left, right) => left.label.localeCompare(right.label)),
];

export function extensionForLanguage(languageName: string): string {
  return languageOptions.find((language) => language.value === languageName)?.extension ?? 'txt';
}

export function detectLanguageFromCode(source: string): string {
  const code = source.trim();
  const lowerCode = code.toLowerCase();

  if (!code) {
    return 'Plain Text';
  }

  if (looksLikeJson(code)) return 'JSON';
  if (/^<!doctype html/i.test(code) || /<\/?(?:html|head|body|main|section|article|div|span|p|a|button|form|input|script|style|template|ul|ol|li|table|svg)\b/i.test(code)) return 'HTML';
  if (/^#!.*\b(?:bash|sh|zsh|fish)\b/.test(code) || /\b(?:sudo|chmod|curl|grep|awk|sed)\b/.test(code)) return 'Shell';
  if (/^#!.*\bpython\b/.test(code) || /\bdef\s+\w+\s*\(|\bfrom\s+\w+\s+import\b|\bimport\s+\w+/.test(code) && /:\s*(?:#.*)?$/m.test(code)) return 'Python';
  if (/^#!.*\b(?:node|deno)\b/.test(code)) return 'JavaScript';
  if (/<\?php/i.test(code) || /\bnamespace\s+[\w\\]+;/.test(code)) return 'PHP';
  if (/\bSELECT\b[\s\S]+\bFROM\b|\bINSERT\s+INTO\b|\bCREATE\s+TABLE\b|\bALTER\s+TABLE\b/i.test(code)) return 'SQL';
  if (/\bfn\s+\w+|let\s+mut\b|impl\s+\w+|use\s+std::|#\[derive\(/.test(code)) return 'Rust';
  if (/\bpackage\s+main\b|\bfunc\s+\w+\s*\(|\bimport\s+"[^"]+"/.test(code)) return 'Go';
  if (/\busing\s+System\b|\bnamespace\s+\w+|\bpublic\s+(?:class|record|interface)\b/.test(code)) return 'C#';
  if (/#include\s*<[^>]+>|\bstd::|\bcout\s*<</.test(code)) return 'C++';
  if (/\bpublic\s+class\s+\w+|\bpublic\s+static\s+void\s+main\s*\(/.test(code)) return 'Java';
  if (/\binterface\s+\w+|\btype\s+\w+\s*=|:\s*(?:string|number|boolean|unknown|Record<)/.test(code)) return 'TypeScript';
  if (/\b(?:import|export)\s+.+from\s+['"]|=>|\bconst\s+\w+\s*=|\bfunction\s+\w+\s*\(/.test(code)) return 'JavaScript';
  if (/\{[\s\S]*:[\s\S]*\}|@[a-z-]+|#[\w-]+\s*\{|(?:^|\n)\.[\w-]+\s*\{/.test(code)) return 'CSS';
  if (/^---\s*$[\s\S]*^---\s*$/m || /^#{1,6}\s+\S/m || /\n[-*]\s+\S/.test(code)) return 'Markdown';
  if (/^[\w.-]+:\s+\S/m && !/[{};]/.test(code)) return 'YAML';
  if (/^\[[\w.-]+\]\s*$/m || /^[\w.-]+\s*=\s*["\d]/m && lowerCode.includes('\n')) return 'TOML';

  return 'Plain Text';
}

function looksLikeJson(code: string): boolean {
  if (!/^\s*[[{]/.test(code)) {
    return false;
  }

  try {
    JSON.parse(code);
    return true;
  } catch {
    return false;
  }
}
