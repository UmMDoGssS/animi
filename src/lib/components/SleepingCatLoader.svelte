<script lang="ts">
    import { onMount } from "svelte";

    interface Props {
        compact?: boolean;
    }

    const frames = [
        [
            "                        ",
            "                        ",
            "                        ",
            "      |\\      *,,,---,,*",
            "ZZZzz /,`.-'`'    -.  ;-;;,_",
            "     |,4-  ) )-,_. ,\\ (  `'-'",
            "    '---''(_/--'  `-'\\_) ",
        ],
        [
            "                        ",
            "                        ",
            "                        ",
            "      |\\      *,,,---,,* ",
            "ZZZz  /,`.-'`'    -.  ;- `:-",
            "     |,4-  ) )-,_. ,\\ (  `'-'",
            "    '---''(_/--'  `-'\\_) ",
        ],
        [
            "                        ",
            "                        ",
            "                        _",
            "      |\\      _,,,---,,`,-`. ",
            "ZZZ   /,`.-'`'    -.  ;'  `~",
            "     |,4-  ) )-,_. ,\\ (  ",
            "    '---''(_/--'  `-'\\_) ",
        ],
        [
            "                        ",
            "                        /;",
            '                       ";',
            "                       :;",
            "      |\\      _,,,---,,`,",
            "ZZ   /,`.-'`'    -.  ;'  ",
            "     |,4-  ) )-,_. ,\\ (  ",
            "    '---''(_/--'  `-'\\_) ",
        ],
        [
            "                        ",
            "        *             ;\\",
            '                       ";',
            "  *                   -:;-",
            "      |\\ *    _,,,---,-`,-",
            "Z     /o`.-'`'    -.  ;'  ",
            "     |,4-  ) )-,_. ,\\ (  ",
            "    '---''(_/--'  `-'\\_) ",
        ],
        [
            "                        ",
            "             MEOW      /\\",
            '      |               -";-',
            "   \\      /           -:;-",
            "      |\\      _,,,---,-`,-",
            "      /o`.-'`'    -.  ;'  ",
            "     |o4-  ) )-,_. ,\\ (  ",
            "    '---''(_/--'  `-'\\_) ",
        ],
        [
            "                        ",
            "        *             ;\\",
            '                       ";',
            "  *                   -:;-",
            "      |\\ *    _,,,---,-`,-",
            "Z     /o`.-'`'    -.  ;'  ",
            "     |,4-  ) )-,_. ,\\ (  ",
            "    '---''(_/--'  `-'\\_) ",
        ],
        [
            "                        ",
            "                        /;",
            '                       ";',
            "                       :;",
            "      |\\      _,,,---,,`,",
            "ZZ   /,`.-'`'    -.  ;'  ",
            "     |,4-  ) )-,_. ,\\ (  ",
            "    '---''(_/--'  `-'\\_) ",
        ],
        [
            "                        ",
            "                        ",
            "                        _",
            "      |\\      _,,,---,,`,-`. ",
            "ZZZ   /,`.-'`'    -.  ;'  `~",
            "     |,4-  ) )-,_. ,\\ (  ",
            "    '---''(_/--'  `-'\\_) ",
        ],
        [
            "                        ",
            "                        ",
            "                        ",
            "      |\\      *,,,---,,* ",
            "ZZZz  /,`.-'`'    -.  ;- `:-",
            "     |,4-  ) )-,_. ,\\ (  `'-'",
            "    '---''(_/--'  `-'\\_) ",
        ],
    ].map((lines) => lines.join("\n"));

    // Keep the long nap from the shell animation, shortened so ordinary loads
    // still have a chance to show the cat waking up.
    const frameDurations = [2400, 300, 300, 250, 200, 450, 200, 250, 300, 900];

    let { compact = false }: Props = $props();
    let frameIndex = $state(0);

    onMount(() => {
        if (window.matchMedia?.("(prefers-reduced-motion: reduce)").matches) {
            return;
        }

        let timeoutId: number;
        const advanceFrame = () => {
            frameIndex = (frameIndex + 1) % frames.length;
            timeoutId = window.setTimeout(
                advanceFrame,
                frameDurations[frameIndex],
            );
        };

        timeoutId = window.setTimeout(advanceFrame, frameDurations[frameIndex]);
        return () => window.clearTimeout(timeoutId);
    });
</script>

<pre aria-hidden="true" data-sleeping-cat-loader class:compact>{frames[
        frameIndex
    ]}</pre>

<style>
    pre {
        width: 35ch;
        min-width: 35ch;
        height: 8.4em;
        margin: 0;
        overflow: hidden;
        color: var(--accent);
        font-family: inherit;
        font-size: clamp(0.45rem, 1.35vw, 0.6rem);
        font-weight: 500;
        line-height: 1.05;
        white-space: pre;
        user-select: none;
    }

    pre.compact {
        font-size: clamp(0.4rem, 1.15vw, 0.52rem);
    }
</style>
