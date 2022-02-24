import React from 'react';
import PropTypes from 'prop-types';
import clsx from 'clsx';
import styles from './index.module.css';

const BlockQuote = ({children, type = "info"}) => (
  <blockquote className={clsx(styles.blockquote, styles[type])}>
    {children}
  </blockquote>
);

BlockQuote.propTypes = {
  children: PropTypes.oneOfType([PropTypes.array, PropTypes.string]).isRequired,
  type: PropTypes.oneOf(["info", "warning", "danger", "note"]),
};

export default BlockQuote;
