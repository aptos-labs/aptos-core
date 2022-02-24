import React, {useEffect, useState} from 'react';
import PropTypes from 'prop-types';

import useBaseUrl from '@docusaurus/useBaseUrl';
import {useColorMode} from '@docusaurus/theme-common';

let counter = 0;

const getImage = (imageDark, imageDarkHover, imageLight, imageLightHover, isHovered,
) => {
  if (!imageLight) {
    return '';
  }

  const {isDarkTheme} = useColorMode();

  const backgroundImage = isDarkTheme && imageDark ? imageDark : imageLight;

  let hoverBackgroundImage = isDarkTheme && imageDarkHover ? imageDarkHover : imageLightHover;
  hoverBackgroundImage = hoverBackgroundImage ? hoverBackgroundImage : backgroundImage;

  const image = isHovered ? hoverBackgroundImage : backgroundImage;

  return useBaseUrl(image);
};

const WithBackgroundImage = ({
  children,
  className,
  imageDark,
  imageDarkHover,
  imageLight,
  imageLightHover,
  ...props
}: WithBackgroundImageProps) => {
  const [isHovered, setIsHovered] = useState(false);
  const images = [imageDark, imageDarkHover, imageLight, imageLightHover];

  const image = getImage(imageDark, imageDarkHover, imageLight, imageLightHover, isHovered);

  const backgroundImageStyle = image
    ? {'backgroundImage': `url('${image}')`}
    : {};
  const imagesToPreload = images.filter(url => url).map(url => useBaseUrl(url));

  useEffect(() => {
    const preloadedImages = imagesToPreload.map(url => {
      const image = new Image();
      image.src = url;
      return image;
    });
    // @ts-ignore
    window.preloadedImages = window.preloadedImages || [];
    // @ts-ignore
    window.preloadedImages[counter++] = preloadedImages;
  }, []);

  return (
    <div
      className={className}
      onMouseEnter={() => setIsHovered(true)}
      onMouseLeave={() => setIsHovered(false)}
      style={backgroundImageStyle}
      {...props}
    >
      {children}
    </div>
  );
};

interface WithBackgroundImageProps {
  children?: PropTypes.ReactElementLike | string;
  imageDark: string;
  imageDarkHover?: string;
  imageLight: string;
  imageLightHover?: string;
  className?: string;
}

export default WithBackgroundImage;
