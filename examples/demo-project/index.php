<?php

declare(strict_types=1);

function hello(string $name): string
{
    return sprintf('Hello, %s!', $name);
}

echo hello('ForgeEdit');
