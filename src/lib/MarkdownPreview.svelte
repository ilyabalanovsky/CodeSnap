<script lang="ts">
  import MarkdownIt from 'markdown-it';

  export let value = '';
  export let lineHeight = 1.55;

  const markdown = new MarkdownIt({
    breaks: false,
    html: false,
    linkify: true,
    typographer: true,
  });

  $: rendered = markdown.render(value);
</script>

<div class="markdown-preview" style={`--line-height: ${lineHeight}`}>
  {@html rendered}
</div>
