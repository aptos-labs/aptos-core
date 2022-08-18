import setuptools

with open("README.md", "r", encoding="utf-8") as fh:
    long_description = fh.read()

setuptools.setup(
    author="Aptos Labs",
    author_email="opensource@aptoslabs.com",
    classifiers=[
        "Programming Language :: Python :: 3",
        "License :: OSI Approved :: Apache Software License",
        "Operating System :: OS Independent",
    ],
    include_package_data=True,
    install_requires=["httpx", "pynacl"],
    long_description=long_description,
    long_description_content_type="text/markdown",
    name="aptos_sdk",
    packages=["aptos_sdk"],
    python_requires=">=3.7",
    url="https://github.com/aptos-labs/aptos-core",
    version="0.2.0",
)
