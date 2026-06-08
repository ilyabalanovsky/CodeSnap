import { toPng } from 'html-to-image';

export async function renderNodeToPngDataUrl(node: HTMLElement): Promise<string> {
  const width = Math.ceil(node.getBoundingClientRect().width);
  const height = Math.ceil(Math.max(node.scrollHeight, node.getBoundingClientRect().height));

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
