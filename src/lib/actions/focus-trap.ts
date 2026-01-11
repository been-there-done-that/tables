/**
 * Action to trap focus within an element.
 * Perfect for dialogs and popovers to ensure tab-navigation stays contained.
 */
export function focusTrap(node: HTMLElement) {
    const handleKeydown = (e: KeyboardEvent) => {
        if (e.key !== 'Tab') return;

        const focusableElements = Array.from(
            node.querySelectorAll<HTMLElement>(
                'button:not([disabled]), [href], input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])'
            )
        );

        if (focusableElements.length === 0) return;

        const firstElement = focusableElements[0];
        const lastElement = focusableElements[focusableElements.length - 1];

        if (e.shiftKey) {
            if (document.activeElement === firstElement) {
                lastElement.focus();
                e.preventDefault();
            }
        } else {
            if (document.activeElement === lastElement) {
                firstElement.focus();
                e.preventDefault();
            }
        }
    };

    node.addEventListener('keydown', handleKeydown, true);

    return {
        destroy() {
            node.removeEventListener('keydown', handleKeydown, true);
        },
    };
}
