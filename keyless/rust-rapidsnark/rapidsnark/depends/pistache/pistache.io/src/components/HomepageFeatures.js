// SPDX-FileCopyrightText: 2021 Andrea Pappacoda
//
// SPDX-License-Identifier: Apache-2.0

import React from 'react';
import clsx from 'clsx';
import styles from './HomepageFeatures.module.css';

const FeatureList = [
  {
    title: 'Modern API',
    Svg: require('../../static/img/undraw_futuristic_interface.svg').default,
    description: (
      <>
        Written in pure C++17 and providing a low-level HTTP abstraction,
        Pistache makes playing with its modern API fun and easy,
        just take a look at the quickstart
      </>
    ),
  },
  {
    title: 'What\'s in the box',
    Svg: require('../../static/img/undraw_accept_request.svg').default,
    description: (
      <>
        <ul>
          <li>A multi-threaded HTTP server to build your APIs</li>
          <li>An asynchronous HTTP client to request APIs</li>
          <li>An HTTP router to dispatch requests to C++ functions</li>
          <li>A REST description DSL to easily define your APIs</li>
          <li>Type-safe headers and MIME types implementation</li>
        </ul>
      </>
    ),
  },
  {
    title: 'Use it',
    Svg: require('../../static/img/undraw_version_control.svg').default,
    description: (
      <>
        <ul>
          <li>Clone it on <a href="https://github.com/pistacheio/pistache">GitHub</a></li>
          <li>Start with the <a href="docs/">quickstart</a></li>
          <li>Read the full user's <a href="docs/http-handler">guide</a></li>
          <li>Have issues with it? Fill an <a href="https://github.com/pistacheio/pistache/issues">issue</a></li>
        </ul>
      </>
    ),
  },
];

function Feature({Svg, title, description}) {
  return (
    <div className={clsx('col col--4')}>
      <div className="text--center">
        <Svg className={styles.featureSvg} alt={title} />
      </div>
      <h3>{title}</h3>
      <p>{description}</p>
    </div>
  );
}

export default function HomepageFeatures() {
  return (
    <section className={styles.features}>
      <div className="container">
        <div className="row">
          {FeatureList.map((props, idx) => (
            <Feature key={idx} {...props} />
          ))}
        </div>
      </div>
    </section>
  );
}
