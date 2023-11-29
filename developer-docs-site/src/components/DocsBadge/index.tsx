// This code oringates from DocVersionBadge in the @docusaurus/theme-classic package.

import React from 'react';
import clsx from 'clsx';

export default function DocBadge({
    text,
    className,
}: { text: string, className: string }): JSX.Element | null {
    return (
        <span
            className={clsx(
                className,
                'badge badge--secondary',
            )}>
            {text}
        </span>
    );
}
