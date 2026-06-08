import { toPng } from 'html-to-image';

export async function renderNodeToPngDataUrl(node: HTMLElement): Promise<string> {
  return toPng(node, {
    cacheBust: true,
    pixelRatio: 2,
    style: {
      transform: 'none',
    },
  });
}

export function downloadDataUrl(dataUrl: string, filename: string): void {
  const link = document.createElement('a');
  link.download = filename;
  link.href = dataUrl;
  link.click();
}

export async function copyDataUrlToClipboard(dataUrl: string): Promise<boolean> {
  if (!('ClipboardItem' in window) || !navigator.clipboard?.write) {
    return false;
  }

  const response = await fetch(dataUrl);
  const blob = await response.blob();
  await navigator.clipboard.write([new ClipboardItem({ [blob.type]: blob })]);
  return true;
}
