import { defineRecipe } from '@chakra-ui/react';
import { fieldRecipe } from '../Input/Input.recipe';

export const textareaRecipe = defineRecipe({ ...fieldRecipe, className: 'chakra-textarea' });
