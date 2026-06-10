import { toPng } from 'html-to-image';

export async function renderNodeToPngDataUrl(node: HTMLElement): Promise<string> {
  const rect = node.getBoundingClientRect();
  const width = Math.ceil(Math.max(rect.width, node.offsetWidth));
  const height = Math.ceil(Math.max(node.scrollHeight, rect.height, node.offsetHeight));

  if (width <= 0 || height <= 0) {
    throw new Error('Snapshot target has no renderable size.');
  }

  return toPng(node, {
    cacheBust: true,
    height,
    pixelRatio: 2,
    style: {
      height: `${height}px`,
      maxHeight: 'none',
      overflow: 'hidden',
      transform: 'none',
      width: `${width}px`,
    },
    width,
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
